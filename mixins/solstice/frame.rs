// Once a frame, after the mouse: Aim Assist.
fn solstice_frame(game: &mut crate::game::Game<'_>) {
    crate::game::solstice::sync_binds(game);
    crate::game::solstice_hud_editor::frame(game);
    crate::game::solstice::block_scan_frame(game);
    crate::game::solstice_render::frame(game);
}
