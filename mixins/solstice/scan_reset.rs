// World lifecycle also covers integrated-server closes; remote hooks cover reconnect/respawn.
fn solstice_scan_reset() {
    crate::game::solstice::clear_render_scans();
}
