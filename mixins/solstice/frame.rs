// Once a frame, after the mouse: Aim Assist.
fn solstice_frame(game: &mut crate::game::Game<'_>) {
    let _ = crate::game::solstice_low_health::update(game);
    crate::game::solstice_render::frame(game);
}
