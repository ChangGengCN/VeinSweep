// 玩家开始破坏方块：交给连锁逻辑（crate::game::veinsweep）。
fn veinsweep_chain(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    canceled: &mut bool,
) {
    crate::game::veinsweep::on_destroy_block(game, pos, state, canceled);
}
