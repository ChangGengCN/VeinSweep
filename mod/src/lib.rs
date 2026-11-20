// VeinSweep：按住潜行挖掉一个方块时，把与它相连的同类方块一起挖掉。
//
// 触发点是 mod API 的 `break_block` 钩子（服务端，对应 NeoForge 的
// `BreakBlockEvent`）；连锁破坏使用 `PlayerRef::destroy_block`，走游戏自己的
// `ServerPlayerGameMode.destroyBlock`：权限、工具判定、掉落、耐久、统计/经验、
// 粒子和音效都与手动挖一块完全一致。
use mod_api::{
    self, BreakBlockEvent, Command, CommandArgumentValue, CommandSource, Flow, Hooks, Host,
    HostBlockStates, RegisterCommandsEvent, RegistrationContext, Registrar, RunModCommandEvent,
    ServerHost, SettingSpec, Text,
};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

const ID: &str = "veinsweep";
/// 黑名单文件名，位于 `<游戏目录>/horizon-mod-data/veinsweep/` 下。
const BLACKLIST_FILE: &str = "blacklist.txt";

/// 正在连锁中的标记：连锁破坏的每个方块都会再次触发 `break_block`，
/// 用它避免无限递归。
static CHAINING: AtomicBool = AtomicBool::new(false);

/// 黑名单缓存；命令修改后整体替换。
static BLACKLIST: Mutex<Option<Arc<HashSet<String>>>> = Mutex::new(None);

/// 最大方块数的可选范围（设置页滑块）。
const LIMIT_MIN: f64 = 8.0;
const LIMIT_MAX: f64 = 256.0;
const LIMIT_STEP: f64 = 8.0;

#[derive(Default)]
struct VeinSweep;

#[mod_api::horizon_mod]
impl Hooks for VeinSweep {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let _ = r.register_setting(SettingSpec::toggle(
            "sneak",
            "veinsweep.setting.sneak",
            true,
        ));
        let _ = r.register_setting(SettingSpec::range(
            "limit",
            "veinsweep.setting.limit",
            LIMIT_MIN,
            LIMIT_MAX,
            LIMIT_STEP,
            32.0,
        ));
        let _ = r.register_setting(SettingSpec::toggle(
            "inventory",
            "veinsweep.setting.inventory",
            false,
        ));
        let _ = r.register_setting(
            SettingSpec::toggle("diagonal", "veinsweep.setting.diagonal", true)
                .tooltip("veinsweep.setting.diagonal.tooltip"),
        );
        let _ = r.register_setting(SettingSpec::range(
            "keep",
            "veinsweep.setting.keep",
            0.0,
            10.0,
            1.0,
            0.0,
        ));
        let _ = r.register_setting(SettingSpec::toggle(
            "creative",
            "veinsweep.setting.creative",
            false,
        ));
    }

    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::new(
            "veinsweep blacklist add <block:resource:minecraft:block>",
        ));
        event.commands.push(Command::new(
            "veinsweep blacklist remove <block:resource:minecraft:block>",
        ));
        event.commands.push(Command::new("veinsweep blacklist list"));
        event.commands.push(Command::new("veinsweep blacklist clear"));
    }

    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        if event.command.literals.first().map(String::as_str) != Some(ID) {
            return Flow::Continue;
        }
        let action = event.command.literals.get(2).map(String::as_str).unwrap_or("");
        let reply = match action {
            "add" => match event.command.argument("block") {
                Some(CommandArgumentValue::Id(block)) => add_block(host, &block.to_string()),
                _ => "用法：/veinsweep blacklist add <方块>".to_owned(),
            },
            "remove" => match event.command.argument("block") {
                Some(CommandArgumentValue::Id(block)) => remove_block(host, &block.to_string()),
                _ => "用法：/veinsweep blacklist remove <方块>".to_owned(),
            },
            "list" => list_blocks(host),
            "clear" => clear_blocks(host),
            _ => "用法：/veinsweep blacklist <add|remove|list|clear>".to_owned(),
        };
        match event.source {
            CommandSource::Player { player, .. } => {
                let _ = player.tell(host, Text::literal(reply));
            }
            _ => {
                let _ = host.commands().feedback(Text::literal(reply));
            }
        }
        Flow::Cancel
    }

    fn break_block(
        &self,
        host: &mut dyn ServerHost,
        event: &mut BreakBlockEvent<'_>,
    ) -> Flow {
        if event.cancelled || !host.on(ID) {
            return Flow::Continue;
        }
        // 连锁中的方块会再次进入这里：放行，让游戏正常破坏。
        if CHAINING.swap(true, Ordering::Relaxed) {
            return Flow::Continue;
        }
        let _guard = ChainGuard;

        let Some(player) = host.players().acting_player() else {
            return Flow::Continue;
        };
        // 旁观者不挖方块。创造模式默认只破一块，需在设置里单独开启连锁；
        // 开启后仍要满足潜行等其余触发条件。
        let game_mode = player.game_mode(host);
        if game_mode == Some(mod_api::GameMode::Spectator) {
            return Flow::Continue;
        }
        if game_mode == Some(mod_api::GameMode::Creative) && !setting_bool(host, "creative", false) {
            return Flow::Continue;
        }
        if setting_bool(host, "sneak", true)
            && !player.is_secondary_use_active(host).unwrap_or(false)
        {
            return Flow::Continue;
        }

        // 连锁只由「潜行 + 破坏方块」触发，与手里拿着什么无关；
        // 并且只在**同种方块**内部扩散：按方块 ID 比较（忽略朝向等状态）。
        let origin = event.state.block().id(host);
        let blacklist = blacklist(host);
        if blacklist.contains(&origin) {
            return Flow::Continue;
        }

        let limit = setting_number(host, "limit", 32.0).clamp(LIMIT_MIN, LIMIT_MAX) as u32;
        let keep = setting_number(host, "keep", 0.0).clamp(0.0, 10.0) as u32;
        let diagonal = setting_bool(host, "diagonal", true);
        let into_inventory = setting_bool(host, "inventory", false);
        let Some(level) = player.level(host) else {
            return Flow::Continue;
        };

        let offsets = neighbour_offsets(diagonal);
        let mut seen: HashSet<[i32; 3]> = HashSet::new();
        seen.insert(event.pos);
        let mut queue: VecDeque<[i32; 3]> = VecDeque::new();
        queue.push_back(event.pos);
        let mut broken = 0u32;

        while let Some(at) = queue.pop_front() {
            if broken >= limit {
                break;
            }
            for offset in offsets.iter().copied() {
                let next = [
                    at[0].saturating_add(offset[0]),
                    at[1].saturating_add(offset[1]),
                    at[2].saturating_add(offset[2]),
                ];
                if !seen.insert(next) || !level.is_loaded(host, next) {
                    continue;
                }
                let Some(state) = level.block_state(host, next) else {
                    continue;
                };
                if host
                    .block_states()
                    .destroy_speed(state)
                    .is_some_and(|speed| speed < 0.0)
                {
                    continue;
                }
                if host
                    .block_states()
                    .has_block_entity(state)
                    .unwrap_or(false)
                {
                    continue;
                }
                let id = state.block().id(host);
                // 只管同种：方块 ID 与起点不同就不扩散（泥土/草方块、原木/树叶等都不互通）。
                // 同一 ID 的不同状态（如原木朝向）仍算同种。
                if id != origin {
                    continue;
                }
                if blacklist.contains(&id) {
                    continue;
                }
                // 保留工具耐久：手持带耐久的物品且剩余耐久低于设定值时立刻停止。
                if !durability_left(host, player, keep) {
                    return Flow::Continue;
                }
                if player.destroy_block(host, next, into_inventory) {
                    broken += 1;
                    queue.push_back(next);
                    if broken >= limit {
                        break;
                    }
                }
            }
        }
        Flow::Continue
    }
}

/// 连锁进行中的标记；离开作用域时清除。
struct ChainGuard;

impl Drop for ChainGuard {
    fn drop(&mut self) {
        CHAINING.store(false, Ordering::Relaxed);
    }
}

/// 设置页的开关值；缺省或非法时用 `default`。
fn setting_bool(host: &dyn Host, key: &str, default: bool) -> bool {
    match host.setting(ID, key).as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => default,
    }
}

/// 设置页的数值；缺省或非法时用 `default`。
fn setting_number(host: &dyn Host, key: &str, default: f64) -> f64 {
    host.setting(ID, key)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

/// 主手物品；空格或空气视为空手。
fn hand_stack(host: &dyn Host, player: mod_api::PlayerRef) -> Option<mod_api::ItemStack> {
    let slot = player.selected_slot(host)? as u32;
    player
        .inventory_slots(host)?
        .into_iter()
        .find(|(index, _)| *index == slot)
        .map(|(_, stack)| stack)
}

/// 剩余耐久是否仍高于保留值；没有耐久概念的物品（或空手）直接放行。
fn durability_left(
    host: &dyn Host,
    player: mod_api::PlayerRef,
    keep: u32,
) -> bool {
    let Some(stack) = hand_stack(host, player) else {
        return true;
    };
    // 方块、食物等没有 `minecraft:max_damage`：宿主可能报成 0，
    // 一律当作“没有耐久”，不能因此拦下连锁。
    let Some(max) = host.max_damage(stack.item()).filter(|max| *max > 0) else {
        return true;
    };
    max.saturating_sub(stack.damage()) > keep
}

/// 六面或含对角的邻居偏移。
fn neighbour_offsets(diagonal: bool) -> Vec<[i32; 3]> {
    let mut offsets = Vec::with_capacity(if diagonal { 26 } else { 6 });
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }
                if !diagonal && (dx as i32).abs() + (dy as i32).abs() + (dz as i32).abs() != 1 {
                    continue;
                }
                offsets.push([dx, dy, dz]);
            }
        }
    }
    offsets
}

/// 读取黑名单（缓存一次；命令修改后整体替换）。
fn blacklist(host: &dyn Host) -> Arc<HashSet<String>> {
    let mut cache = lock_cache();
    if let Some(set) = cache.as_ref() {
        return Arc::clone(set);
    }
    let set = Arc::new(load_blacklist(host));
    *cache = Some(Arc::clone(&set));
    set
}

fn lock_cache() -> MutexGuard<'static, Option<Arc<HashSet<String>>>> {
    BLACKLIST.lock().unwrap_or_else(|poison| poison.into_inner())
}

fn load_blacklist(host: &dyn Host) -> HashSet<String> {
    let mut set = HashSet::new();
    let Ok(path) = mod_api::ConfigPath::new(BLACKLIST_FILE) else {
        return set;
    };
    let Ok(bytes) = host.data_files().read(&path) else {
        return set;
    };
    if let Ok(text) = String::from_utf8(bytes) {
        for line in text.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                set.insert(line.to_owned());
            }
        }
    }
    set
}

fn save_blacklist(host: &dyn Host, set: &HashSet<String>) {
    let mut names: Vec<&str> = set.iter().map(String::as_str).collect();
    names.sort_unstable();
    let mut text = names.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    if let Ok(path) = mod_api::ConfigPath::new(BLACKLIST_FILE) {
        let _ = host.data_files().write(&path, text.as_bytes());
    }
}

fn add_block(host: &mut dyn ServerHost, block: &str) -> String {
    let mut set = (*blacklist(host)).clone();
    if !set.insert(block.to_owned()) {
        return format!("{block} 已在黑名单中。");
    }
    save_blacklist(host, &set);
    *lock_cache() = Some(Arc::new(set));
    format!("已把 {block} 加入黑名单，它不再参与连锁。")
}

fn remove_block(host: &mut dyn ServerHost, block: &str) -> String {
    let mut set = (*blacklist(host)).clone();
    if !set.remove(block) {
        return format!("{block} 不在黑名单中。");
    }
    save_blacklist(host, &set);
    *lock_cache() = Some(Arc::new(set));
    format!("已把 {block} 移出黑名单。")
}

fn list_blocks(host: &mut dyn ServerHost) -> String {
    let set = blacklist(host);
    if set.is_empty() {
        return "黑名单为空。".to_owned();
    }
    let mut names: Vec<&str> = set.iter().map(String::as_str).collect();
    names.sort_unstable();
    format!("黑名单（{} 个）：{}", names.len(), names.join(", "))
}

fn clear_blocks(host: &mut dyn ServerHost) -> String {
    let set = blacklist(host);
    if set.is_empty() {
        return "黑名单已经为空。".to_owned();
    }
    save_blacklist(host, &HashSet::new());
    *lock_cache() = Some(Arc::new(HashSet::new()));
    "已清空黑名单。".to_owned()
}
