// VeinSweep：按住触发键（默认左 Shift，可在 选项 → 控制 中改）挖掉一个方块时，
// 把与它相连的同类方块一起挖掉。
//
// 触发点是 mod API 的 `break_block` 钩子（服务端，对应 NeoForge 的
// `BreakBlockEvent`）；连锁破坏使用 `PlayerRef::destroy_block`，走游戏自己的
// `ServerPlayerGameMode.destroyBlock`：权限、工具判定、掉落、耐久、统计/经验、
// 粒子和音效都与手动挖一块完全一致。
use mod_api::{
    self, BreakBlockEvent, ClientHost, ClientTickPostEvent, Command, CommandArgumentValue,
    CommandSource, Flow, Hooks, Host, HostBlockStates, KeyConflictContext, KeyRegistration,
    RegisterCommandsEvent, RegistrationContext, Registrar, RunModCommandEvent, ServerHost,
    SettingSpec, Text,
};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

const ID: &str = "veinsweep";
/// 黑名单文件名，位于 `<游戏目录>/horizon-mod-data/veinsweep/` 下。
const BLACKLIST_FILE: &str = "blacklist.txt";

/// 正在连锁中的标记：连锁破坏的每个方块都会再次触发 `break_block`，
/// 用它避免无限递归。
static CHAINING: AtomicBool = AtomicBool::new(false);

/// 黑名单缓存；命令修改后整体替换。
static BLACKLIST: Mutex<Option<Arc<HashSet<String>>>> = Mutex::new(None);

/// 触发键的映射 id 与默认键。默认绑到潜行键的位置（原版潜行的默认键是左 Shift），
/// 玩家可在 选项 → 控制 → VeinSweep 里改成任意键盘键或鼠标键；按住它破坏方块才会连锁。
///
/// 这里用具体的物理键而不是引用原版 `key.sneak` 映射：按键注册 API 只接受键名，
/// 不支持别名到另一条映射，所以默认值就是左 Shift；玩家若另行改了原版潜行键，
/// 这条绑定不会跟着变，需要在控制页单独改。
const TRIGGER_MAPPING: &str = "trigger";
const TRIGGER_DEFAULT_KEY: &str = "key.keyboard.left.shift";

/// 客户端每 tick 采到的触发键状态；服务端的 `break_block` 读它。
/// 单人/LAN 主机里客户端与服务端同进程，静态变量对两侧都可见。
static TRIGGER_HELD: AtomicBool = AtomicBool::new(false);

/// 按键状态属于哪名玩家（本机客户端的玩家 UUID）。
/// LAN 里服务端会收到多名玩家的破坏事件，用它把触发键限定在本机玩家身上，
/// 避免主机按着键时远程玩家的方块也被连锁。
static TRIGGER_OWNER: Mutex<Option<u128>> = Mutex::new(None);

/// 最大方块数的可选范围（设置页滑块）。
const LIMIT_MIN: f64 = 8.0;
const LIMIT_MAX: f64 = 256.0;
const LIMIT_STEP: f64 = 8.0;

/// 普通方块（泥土、石头、沙子……）随机扩散的半径，固定值。
///
/// 取值 4：以起点为中心是 9×9×9 的立方体，默认上限 32 块只占其中很小一部分，
/// 形状不会被盒子边缘切得方正，也不会像 6 以上那样在开阔地形挖出过大的洞；
/// 上调“最大方块数”后仍有足够空间，不会因范围过小而频繁触顶。
const SCATTER_RADIUS: i32 = 4;

/// 矿脉/原木用固定 26 向（整块 3×3×3 邻域）连通。
const NEIGHBOURS_26: [[i32; 3]; 26] = {
    let mut offsets = [[0i32; 3]; 26];
    let mut n = 0;
    let mut dx = -1;
    while dx <= 1 {
        let mut dy = -1;
        while dy <= 1 {
            let mut dz = -1;
            while dz <= 1 {
                if !(dx == 0 && dy == 0 && dz == 0) {
                    offsets[n] = [dx, dy, dz];
                    n += 1;
                }
                dz += 1;
            }
            dy += 1;
        }
        dx += 1;
    }
    offsets
};

/// 普通方块随机扩散用六面相邻，保证挖出来的是一整块连着的区域。
const NEIGHBOURS_6: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

#[derive(Default)]
struct VeinSweep;

#[mod_api::horizon_mod]
impl Hooks for VeinSweep {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        // 连锁破坏总开关：关闭后挖方块只破坏手动那一个。
        let _ = r.register_setting(
            SettingSpec::toggle("enabled", "veinsweep.setting.enabled", true)
                .tooltip("veinsweep.setting.enabled.tooltip"),
        );
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
        // 触发键：列在 选项 → 控制 → VeinSweep 下，默认左 Shift；
        // 玩家可改成任意键盘键或鼠标键。
        r.register_key_mapping(
            KeyRegistration::new(TRIGGER_MAPPING, TRIGGER_DEFAULT_KEY)
                .label("veinsweep.key.trigger")
                .category("veinsweep.key.category")
                .context(KeyConflictContext::InGame),
        );
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPostEvent) {
        // 只有“正在游玩、且没有打开界面”时才把按键算作按下；
        // 其余时候清零，避免退出界面后残留按下状态。
        let held = host.on(ID) && host.playing() && host.is_down(ID, TRIGGER_MAPPING);
        TRIGGER_HELD.store(held, Ordering::Relaxed);
        // 用 UUID 记住按键属于本机哪名玩家：客户端读不到玩家名，UUID 两端一致。
        let owner = if held {
            host.local_player().map(|player| player.uuid())
        } else {
            None
        };
        *TRIGGER_OWNER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = owner;
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
                _ => Text::translate("veinsweep.command.usage.add", Vec::new()),
            },
            "remove" => match event.command.argument("block") {
                Some(CommandArgumentValue::Id(block)) => remove_block(host, &block.to_string()),
                _ => Text::translate("veinsweep.command.usage.remove", Vec::new()),
            },
            "list" => list_blocks(host),
            "clear" => clear_blocks(host),
            _ => Text::translate("veinsweep.command.usage", Vec::new()),
        };
        match event.source {
            CommandSource::Player { player, .. } => {
                let _ = player.tell(host, reply);
            }
            _ => {
                let _ = host.commands().feedback(reply);
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
        // 总开关：关闭时连锁不生效，手动挖掉的方块照常破坏。
        if !setting_bool(host, "enabled", true) {
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
        // 开启后仍要满足触发条件。
        let game_mode = player.game_mode(host);
        if game_mode == Some(mod_api::GameMode::Spectator) {
            return Flow::Continue;
        }
        if game_mode == Some(mod_api::GameMode::Creative) && !setting_bool(host, "creative", false) {
            return Flow::Continue;
        }
        // 按住触发键才连锁；没按只破坏手动那一个。与手里拿着什么无关。
        // 按键状态来自本机客户端，因此只对本机玩家生效。
        if !TRIGGER_HELD.load(Ordering::Relaxed) || !trigger_owner_is(player) {
            return Flow::Continue;
        }

        // 连锁只由「触发条件 + 破坏方块」触发，与手里拿着什么无关；
        // 并且只在**同种方块**内部扩散：按方块 ID 比较（忽略朝向等状态）。
        let origin = event.state.block().id(host);
        let blacklist = blacklist(host);
        if blacklist.contains(&origin) {
            return Flow::Continue;
        }

        let limit = setting_number(host, "limit", 32.0).clamp(LIMIT_MIN, LIMIT_MAX) as u32;
        let keep = setting_number(host, "keep", 0.0).clamp(0.0, 10.0) as u32;
        let into_inventory = setting_bool(host, "inventory", false);
        let Some(level) = player.level(host) else {
            return Flow::Continue;
        };

        if is_vein_block(&origin) {
            // 矿脉、原木：“成脉”的方块固定按 26 向（整个 3×3×3 邻域）连通，
            // 不再受“含对角连通”开关影响，一直挖到尽头，只受上限约束。
            chain_vein(
                host,
                player,
                level,
                event.pos,
                &origin,
                &blacklist,
                limit,
                keep,
                into_inventory,
            );
        } else {
            // 泥土、石头、沙子：在固定半径内随机生长出一块连通的区域，
            // 每次形状都不同；范围与上限都是硬约束。
            chain_scatter(
                host,
                player,
                level,
                event.pos,
                &origin,
                &blacklist,
                limit,
                keep,
                into_inventory,
                SCATTER_RADIUS,
            );
        }
        Flow::Continue
    }
}

/// 矿脉/原木的判定：按方块 ID 的最后一段做后缀匹配。
///
/// 矿石（`*_ore`、远古残骸）和原木类（`*_log`、`*_wood`、`*_stem`、`*_hyphae`）
/// 是成脉分布的方块，固定用 26 向连通；其余方块走随机扩散。
fn is_vein_block(id: &str) -> bool {
    let name = id.rsplit(':').next().unwrap_or(id);
    name == "ancient_debris"
        || name.ends_with("_ore")
        || name.ends_with("_log")
        || name.ends_with("_wood")
        || name.ends_with("_stem")
        || name.ends_with("_hyphae")
}

/// 这个方块能不能作为连锁目标（不能破坏、带方块实体、黑名单里的都不行）。
fn can_chain(
    host: &dyn Host,
    level: mod_api::LevelRef,
    pos: [i32; 3],
    origin: &str,
    blacklist: &HashSet<String>,
) -> bool {
    if blacklist.contains(origin) || !level.is_loaded(host, pos) {
        return false;
    }
    let Some(state) = level.block_state(host, pos) else {
        return false;
    };
    if host
        .block_states()
        .destroy_speed(state)
        .is_some_and(|speed| speed < 0.0)
    {
        return false;
    }
    if host
        .block_states()
        .has_block_entity(state)
        .unwrap_or(false)
    {
        return false;
    }
    state.block().id(host) == origin
}

/// 矿脉模式：固定 26 向 BFS，把整条同种矿脉挖完（上限是硬约束）。
#[allow(clippy::too_many_arguments)]
fn chain_vein(
    host: &mut dyn ServerHost,
    player: mod_api::PlayerRef,
    level: mod_api::LevelRef,
    origin_pos: [i32; 3],
    origin: &str,
    blacklist: &HashSet<String>,
    limit: u32,
    keep: u32,
    into_inventory: bool,
) {
    let mut seen: HashSet<[i32; 3]> = HashSet::new();
    seen.insert(origin_pos);
    let mut queue: VecDeque<[i32; 3]> = VecDeque::new();
    queue.push_back(origin_pos);
    let mut broken = 0u32;

    while let Some(at) = queue.pop_front() {
        for offset in NEIGHBOURS_26.iter().copied() {
            if broken >= limit {
                return;
            }
            let next = [
                at[0].saturating_add(offset[0]),
                at[1].saturating_add(offset[1]),
                at[2].saturating_add(offset[2]),
            ];
            if !seen.insert(next) || !can_chain(host, level, next, origin, blacklist) {
                continue;
            }
            if !durability_left(host, player, keep) {
                return;
            }
            if player.destroy_block(host, next, into_inventory) {
                broken += 1;
                queue.push_back(next);
            }
        }
    }
}

/// 普通方块模式：以起点为中心、半边长 `radius` 的立方体内，从起点向外随机生长，
/// 直到取到随机目标块数（不超过上限）。只走六面相邻，保证挖出的是一整块连通区域；
/// 随机顺序让每次形状都不同，而立方体范围保证不会出现夸张的大洞。
#[allow(clippy::too_many_arguments)]
fn chain_scatter(
    host: &mut dyn ServerHost,
    player: mod_api::PlayerRef,
    level: mod_api::LevelRef,
    origin_pos: [i32; 3],
    origin: &str,
    blacklist: &HashSet<String>,
    limit: u32,
    keep: u32,
    into_inventory: bool,
    radius: i32,
) {
    let mut rng = Rng::new(random_seed(origin_pos, origin));
    // 目标块数在 60%～100% 上限之间随机，避免每次都顶满上限、形状雷同。
    let low = ((limit as f64) * 0.6).ceil() as u32;
    let low = low.max(1).min(limit);
    let span = (limit - low + 1) as usize;
    let target = low + rng.below(span) as u32;

    let mut chosen: HashSet<[i32; 3]> = HashSet::new();
    chosen.insert(origin_pos);
    let mut frontier: Vec<[i32; 3]> = Vec::new();
    push_scatter_neighbours(
        host,
        level,
        origin_pos,
        origin_pos,
        origin,
        blacklist,
        radius,
        &chosen,
        &mut frontier,
    );

    let mut broken = 0u32;
    while broken < target && !frontier.is_empty() {
        let index = rng.below(frontier.len());
        let next = frontier.swap_remove(index);
        if chosen.contains(&next) {
            continue;
        }
        if !can_chain(host, level, next, origin, blacklist) {
            continue;
        }
        if !durability_left(host, player, keep) {
            return;
        }
        if player.destroy_block(host, next, into_inventory) {
            chosen.insert(next);
            broken += 1;
            push_scatter_neighbours(
                host,
                level,
                next,
                origin_pos,
                origin,
                blacklist,
                radius,
                &chosen,
                &mut frontier,
            );
        }
    }
}

/// 把 `at` 周围仍在立方体范围内、尚未入队的六面邻居加进随机前沿。
#[allow(clippy::too_many_arguments)]
fn push_scatter_neighbours(
    host: &dyn Host,
    level: mod_api::LevelRef,
    at: [i32; 3],
    center: [i32; 3],
    origin: &str,
    blacklist: &HashSet<String>,
    radius: i32,
    chosen: &HashSet<[i32; 3]>,
    frontier: &mut Vec<[i32; 3]>,
) {
    for offset in NEIGHBOURS_6.iter().copied() {
        let next = [
            at[0].saturating_add(offset[0]),
            at[1].saturating_add(offset[1]),
            at[2].saturating_add(offset[2]),
        ];
        if chosen.contains(&next) || !within_box(next, center, radius) {
            continue;
        }
        if !can_chain(host, level, next, origin, blacklist) {
            continue;
        }
        frontier.push(next);
    }
}

fn within_box(pos: [i32; 3], center: [i32; 3], radius: i32) -> bool {
    (pos[0] - center[0]).abs() <= radius
        && (pos[1] - center[1]).abs() <= radius
        && (pos[2] - center[2]).abs() <= radius
}

/// 一个很小的 xorshift64 随机数发生器，避免额外依赖。
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u64() % bound as u64) as usize
        }
    }
}

/// 用当前时间、起点位置和方块 ID 混出一个种子：同样的位置连挖两次也会得到不同形状。
fn random_seed(pos: [i32; 3], origin: &str) -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut hash = nanos ^ 0x9E37_79B9_7F4A_7C15;
    for value in pos {
        hash = hash.rotate_left(7) ^ (value as i64 as u64).wrapping_mul(0x0100_0000_01B3);
    }
    for byte in origin.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(byte));
    }
    hash
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

/// 触发键状态是否属于这次破坏的玩家。
///
/// 客户端只采样本机玩家的按键；LAN 里服务端会收到其他玩家的破坏事件，
/// 这里用玩家 UUID 把触发限定在本机玩家身上，避免主机的按键带动别人。
fn trigger_owner_is(player: mod_api::PlayerRef) -> bool {
    TRIGGER_OWNER
        .lock()
        .map(|owner| *owner == Some(player.uuid()))
        .unwrap_or(false)
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

fn add_block(host: &mut dyn ServerHost, block: &str) -> Text {
    let mut set = (*blacklist(host)).clone();
    if !set.insert(block.to_owned()) {
        return Text::translate(
            "veinsweep.command.present",
            vec![Text::literal(block)],
        );
    }
    save_blacklist(host, &set);
    *lock_cache() = Some(Arc::new(set));
    Text::translate("veinsweep.command.added", vec![Text::literal(block)])
}

fn remove_block(host: &mut dyn ServerHost, block: &str) -> Text {
    let mut set = (*blacklist(host)).clone();
    if !set.remove(block) {
        return Text::translate(
            "veinsweep.command.absent",
            vec![Text::literal(block)],
        );
    }
    save_blacklist(host, &set);
    *lock_cache() = Some(Arc::new(set));
    Text::translate("veinsweep.command.removed", vec![Text::literal(block)])
}

fn list_blocks(host: &mut dyn ServerHost) -> Text {
    let set = blacklist(host);
    if set.is_empty() {
        return Text::translate("veinsweep.command.list.empty", Vec::new());
    }
    let mut names: Vec<&str> = set.iter().map(String::as_str).collect();
    names.sort_unstable();
    Text::translate(
        "veinsweep.command.list",
        vec![
            Text::literal(names.len().to_string()),
            Text::literal(names.join(", ")),
        ],
    )
}

fn clear_blocks(host: &mut dyn ServerHost) -> Text {
    let set = blacklist(host);
    if set.is_empty() {
        return Text::translate("veinsweep.command.clear.empty", Vec::new());
    }
    save_blacklist(host, &HashSet::new());
    *lock_cache() = Some(Arc::new(HashSet::new()));
    Text::translate("veinsweep.command.cleared", Vec::new())
}
