// VeinSweep 设置页的按钮。
fn veinsweep_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::veinsweep::settings(game, mod_id, buttons);
}
