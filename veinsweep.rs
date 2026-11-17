// VeinSweep：用镐/斧挖掉一个矿石或原木时，把与它相连的同类方块一起挖掉。
//
// 触发点是 Game::native_destroy_block（玩家开始破坏一个方块），
// 破坏队友方块时走 Game::native_player_break_block，也就是游戏自己的
// ServerPlayerGameMode.destroyBlock：玩家的权限检查、正确工具判定、
// 掉落、耐久消耗、统计/经验、粒子和音效都按正常挖矿处理。
use crate::game::Game;
use horizon_client::items::ItemStack;
use horizon_world::registry::BlockStateId;
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) const ID: &str = "veinsweep";
const TITLE: &str = "VeinSweep 0.2.1";
const DESCRIPTION: &str =
    "连锁挖掘：按住潜行，用镐或斧挖掉一个矿石/原木，就会把与它相连的同类方块一起挖掉。\
     默认只处理矿石与原木；“基础方块”（石头/泥土/沙子类）可在设置里统一开启。\
     空手也能连锁原版空手可获得掉落的方块；数量有上限，掉落与耐久按正常挖矿计算。";

/// 正在连锁中的标记：连锁破坏的每个方块都会再次触发 destroy_block 钩子，
/// 用它避免无限递归。
static CHAINING: AtomicBool = AtomicBool::new(false);

/// 视为矿石的方块标签（存在的标签才算；`c:`/`forge:` 是通用标签）。
const ORE_TAGS: [&str; 4] = [
    "minecraft:ores",
    "c:ores",
    "forge:ores",
    "neoforge:ores",
];

/// 视为原木/菌柄的方块标签。
const LOG_TAGS: [&str; 4] = [
    "minecraft:logs",
    "c:logs",
    "forge:logs",
    "neoforge:logs",
];

/// 石头类的标签（原版自然石头、深板岩、下界岩等）。
const STONE_TAGS: [&str; 2] = [
    "minecraft:base_stone_overworld",
    "minecraft:base_stone_nether",
];

/// 石头类的额外精确方块（不在上面的标签里）。
const STONE_BLOCKS: [&str; 3] = [
    "minecraft:end_stone",
    "minecraft:calcite",
    "minecraft:smooth_basalt",
];

/// 泥土类的额外精确方块（`minecraft:dirt` 标签只含三种）。
const DIRT_BLOCKS: [&str; 7] = [
    "minecraft:grass_block",
    "minecraft:podzol",
    "minecraft:mycelium",
    "minecraft:moss_block",
    "minecraft:mud",
    "minecraft:muddy_mangrove_roots",
    "minecraft:soul_soil",
];

/// 沙子类的额外精确方块（`minecraft:sand` 标签之外）。
const SAND_BLOCKS: [&str; 2] = [
    "minecraft:gravel",
    "minecraft:suspicious_gravel",
];

/// 连锁范围：矿石/原木按同名匹配，基础方块按类别匹配。
#[derive(Clone, Copy)]
enum Scope {
    Exact,
    Stone,
    Dirt,
    Sand,
}

impl Scope {
    /// `here` 是否属于本次连锁范围。
    fn matches(self, game: &Game<'_>, origin: &str, here: &str) -> bool {
        match self {
            Scope::Exact => here == origin,
            Scope::Stone => is_stone_like(game, here),
            Scope::Dirt => is_dirt_like(game, here),
            Scope::Sand => is_sand_like(game, here),
        }
    }
}

/// 最大方块数的可选值（设置页循环切换）。
const LIMITS: [u32; 6] = [8, 16, 32, 64, 128, 256];

/// 进入连锁：已有一层连锁在进行时返回 None。
struct Guard;

impl Guard {
    fn enter() -> Option<Guard> {
        if CHAINING.swap(true, Ordering::Relaxed) {
            None
        } else {
            Some(Guard)
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        CHAINING.store(false, Ordering::Relaxed);
    }
}

/// Options > Mods 里的一行。
pub(super) fn list(mods: &mut Vec<crate::mod_api::NativeMod>) {
    mods.push(crate::mod_api::NativeMod {
        id: ID.to_owned(),
        title: TITLE.to_owned(),
        description: DESCRIPTION.to_owned(),
        keys: Vec::new(),
    });
}

/// 设置页的按钮。
pub(super) fn settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != ID {
        return;
    }
    buttons.push(format!(
        "触发方式：{}",
        if flag(game, "sneak", true) {
            "潜行时"
        } else {
            "总是"
        }
    ));
    buttons.push(format!("最大方块数：{}", limit(game)));
    buttons.push(format!(
        "掉落方式：{}",
        if flag(game, "inventory", false) {
            "直接入包"
        } else {
            "世界散落"
        }
    ));
    buttons.push(format!(
        "连通范围：{}",
        if flag(game, "diagonal", false) {
            "含对角（26 向）"
        } else {
            "六面（6 向）"
        }
    ));
    buttons.push(format!("保留工具耐久：{}", keep_durability(game)));
    buttons.push(format!(
        "空手触发：{}",
        if flag(game, "empty_hand", true) {
            "允许"
        } else {
            "禁止"
        }
    ));
    buttons.push(format!(
        "基础方块：{}",
        if basic_blocks(game) { "允许" } else { "禁止" }
    ));
}

/// 设置页按钮点击。
pub(super) fn settings_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID {
        return;
    }
    match button {
        0 => set_flag(game, "sneak", !flag(game, "sneak", true)),
        1 => {
            let current = limit(game);
            let next = LIMITS
                .iter()
                .copied()
                .find(|v| *v > current)
                .unwrap_or(LIMITS[0]);
            game.native_set_setting(ID, "limit", &next.to_string());
        }
        2 => set_flag(game, "inventory", !flag(game, "inventory", false)),
        3 => set_flag(game, "diagonal", !flag(game, "diagonal", false)),
        4 => {
            let next = if keep_durability(game) >= 10 {
                0
            } else {
                keep_durability(game) + 1
            };
            game.native_set_setting(ID, "keep", &next.to_string());
        }
        5 => set_flag(game, "empty_hand", !flag(game, "empty_hand", true)),
        6 => set_flag(game, "basic", !basic_blocks(game)),
        _ => {}
    }
}

/// 玩家开始破坏 `pos` 处的 `state`：把相连的同类方块也按玩家挖矿挖掉。
pub(super) fn on_destroy_block(
    game: &mut Game<'_>,
    pos: [i32; 3],
    state: BlockStateId,
    canceled: &mut bool,
) {
    if *canceled || CHAINING.load(Ordering::Relaxed) || !game.native_on(ID) {
        return;
    }
    let Some(player) = game.native_acting_player() else {
        return;
    };
    // 旁观者不挖方块；创造模式允许（方便清理，但没有掉落和耐久）。
    if player.game_mode == 3 {
        return;
    }
    if flag(game, "sneak", true) && !player.sneaking {
        return;
    }

    let basic = basic_blocks(game);
    let (origin, requires_tool) = {
        let facts = game.res.facts().get(state);
        (facts.name.clone(), facts.requires_tool)
    };
    let Some(scope) = chain_scope(game, &origin, basic) else {
        return;
    };
    // 空手：主手为空（或空气）时按空手处理。
    let held = game
        .held()
        .cloned()
        .filter(|stack| stack.count > 0 && &*stack.item != "minecraft:air");
    let bare_hand = held.is_none();
    if bare_hand && !flag(game, "empty_hand", true) {
        return;
    }
    if !tool_matches(game, held.as_ref(), &origin, requires_tool) {
        return;
    }

    let creative = game.creative;
    let max_blocks = limit(game);
    let diagonal = flag(game, "diagonal", false);
    let into_inventory = flag(game, "inventory", false);
    let keep = keep_durability(game);
    let dimension = player.dimension.clone();
    let uuid = player.uuid;

    // 连锁期间的每次破坏都会再进到这里：直接放行，让游戏正常破坏。
    let Some(_guard) = Guard::enter() else {
        return;
    };

    let offsets = neighbour_offsets(diagonal);
    let mut seen: HashSet<[i32; 3]> = HashSet::new();
    seen.insert(pos);
    let mut queue: VecDeque<[i32; 3]> = VecDeque::new();
    queue.push_back(pos);
    let mut broken = 0u32;

    while let Some(at) = queue.pop_front() {
        if broken >= max_blocks {
            break;
        }
        for offset in &offsets {
            let next = [
                at[0].saturating_add(offset[0]),
                at[1].saturating_add(offset[1]),
                at[2].saturating_add(offset[2]),
            ];
            if !seen.insert(next) {
                continue;
            }
            if !game.world.loaded(next[0], next[2]) {
                continue;
            }
            let here_state = game.world.block(next[0], next[1], next[2]);
            let here = {
                let facts = game.res.facts().get(here_state);
                if facts.block_entity || facts.hardness < 0.0 {
                    continue;
                }
                facts.name.clone()
            };
            if !scope.matches(game, &origin, &here) {
                continue;
            }
            // 区域保护等模组可以拒绝这次改动。
            if !game.native_may_build(uuid, &here, &dimension, next, "break") {
                continue;
            }
            if !creative && !bare_hand {
                let Some(held) = game.held() else {
                    return;
                };
                if remaining_durability(game, held) <= keep {
                    return;
                }
            }
            if game.native_player_break_block(uuid, next, into_inventory) {
                broken += 1;
                queue.push_back(next);
                if broken >= max_blocks {
                    break;
                }
            }
        }
    }
}

/// 起点方块决定本次连锁的范围；不在范围内时返回 None。
fn chain_scope(game: &Game<'_>, name: &str, basic: bool) -> Option<Scope> {
    let items = game.res.items();
    if ORE_TAGS.iter().any(|tag| items.block_in_tag(tag, name)) {
        return Some(Scope::Exact);
    }
    if LOG_TAGS.iter().any(|tag| items.block_in_tag(tag, name)) {
        return Some(Scope::Exact);
    }
    let path = name.rsplit(':').next().unwrap_or(name);
    if path.ends_with("_ore")
        || path.ends_with("_log")
        || path.ends_with("_wood")
        || path.ends_with("_stem")
        || path.ends_with("_hyphae")
    {
        return Some(Scope::Exact);
    }
    if !basic {
        return None;
    }
    if is_stone_like(game, name) {
        return Some(Scope::Stone);
    }
    if is_dirt_like(game, name) {
        return Some(Scope::Dirt);
    }
    if is_sand_like(game, name) {
        return Some(Scope::Sand);
    }
    None
}

/// 设置页的“基础方块”开关；旧版本的 stone/dirt/sand 任意一个为 true 时视为开启。
fn basic_blocks(game: &Game<'_>) -> bool {
    match game.native_setting(ID, "basic").as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => ["stone", "dirt", "sand"]
            .iter()
            .any(|key| matches!(game.native_setting(ID, key).as_deref(), Some("true"))),
    }
}

/// 石头类：原版自然石头、深板岩、下界岩，以及按命名判断的模组石头。
fn is_stone_like(game: &Game<'_>, name: &str) -> bool {
    let items = game.res.items();
    let path = name.rsplit(':').next().unwrap_or(name);
    STONE_TAGS.iter().any(|tag| items.block_in_tag(tag, name))
        || STONE_BLOCKS.contains(&name)
        || path.ends_with("_stone")
}

/// 泥土类：泥土标签、草方块等自然地表，以及按命名判断的模组泥土。
fn is_dirt_like(game: &Game<'_>, name: &str) -> bool {
    let path = name.rsplit(':').next().unwrap_or(name);
    game.res.items().block_in_tag("minecraft:dirt", name)
        || DIRT_BLOCKS.contains(&name)
        || path.ends_with("_dirt")
}

/// 沙子类：沙子标签、沙砾，以及按命名判断的模组沙子/沙砾。
fn is_sand_like(game: &Game<'_>, name: &str) -> bool {
    let items = game.res.items();
    let path = name.rsplit(':').next().unwrap_or(name);
    items.block_in_tag("minecraft:sand", name)
        || SAND_BLOCKS.contains(&name)
        || path.ends_with("_sand")
        || path.ends_with("_gravel")
}

/// 手持工具是否与方块匹配：镐挖矿石、斧砍原木。
fn tool_matches(game: &Game<'_>, held: Option<&ItemStack>, block: &str, requires_tool: bool) -> bool {
    let items = game.res.items();
    let Some(held) = held else {
        // 空手：只有原版空手也能获得掉落的方块才连锁（不需要正确工具）。
        return !requires_tool;
    };
    let item = &*held.item;
    if items.block_in_tag("minecraft:mineable/pickaxe", block)
        && !items.item_in_tag("minecraft:pickaxes", item)
    {
        return false;
    }
    if items.block_in_tag("minecraft:mineable/axe", block)
        && !items.item_in_tag("minecraft:axes", item)
    {
        return false;
    }
    if items.block_in_tag("minecraft:mineable/shovel", block)
        && !items.item_in_tag("minecraft:shovels", item)
    {
        return false;
    }
    if requires_tool && !items.correct_for_drops(Some(held), block) {
        return false;
    }
    true
}

/// 工具还能承受多少次挖掘（没有耐久的工具视为无限）。
fn remaining_durability(game: &Game<'_>, stack: &ItemStack) -> u32 {
    let max = game.res.items().mining_damage_capacity(stack);
    if max == 0 {
        return u32::MAX;
    }
    max.saturating_sub(stack.damage)
}

fn limit(game: &Game<'_>) -> u32 {
    number(game, "limit", 32).clamp(1, 512)
}

fn keep_durability(game: &Game<'_>) -> u32 {
    number(game, "keep", 2).min(64)
}

fn flag(game: &Game<'_>, key: &str, default: bool) -> bool {
    match game.native_setting(ID, key).as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => default,
    }
}

fn set_flag(game: &Game<'_>, key: &str, value: bool) {
    game.native_set_setting(ID, key, if value { "true" } else { "false" });
}

fn number(game: &Game<'_>, key: &str, default: u32) -> u32 {
    game.native_setting(ID, key)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn neighbour_offsets(diagonal: bool) -> Vec<[i32; 3]> {
    let mut offsets = Vec::with_capacity(if diagonal { 26 } else { 6 });
    for x in -1i32..=1 {
        for y in -1i32..=1 {
            for z in -1i32..=1 {
                let distance = x.abs() + y.abs() + z.abs();
                if distance == 0 {
                    continue;
                }
                if diagonal || distance == 1 {
                    offsets.push([x, y, z]);
                }
            }
        }
    }
    offsets
}
