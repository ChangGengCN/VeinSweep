// VeinSweep 设置页按钮被点击。
fn veinsweep_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::veinsweep::settings_click(game, mod_id, button);
}
