#!/usr/bin/env python3
"""Test the production HUD model and extracted release/close lifecycle adapters."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rustc", default=os.environ.get("RUSTC", "rustc"))
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    source = root / "mixins/solstice/hud_layout.rs"
    runtime = (root / "mixins/solstice/runtime.rs").read_text(encoding="utf-8")
    start = runtime.index("pub(super) fn close_gui(")
    close_gui = runtime[start:runtime.index("// A slider's value", start)]
    adapter = (root / "mixins/solstice/hud_editor.rs").read_text(encoding="utf-8")
    start = adapter.index("pub(super) fn release(")
    release = adapter[start:adapter.index("pub(super) fn start_frame(", start)]
    # Compile the actual runtime body, not a duplicate of its capture condition.
    fixture = r'''
mod close_gui_tests {
    use super::hud_layout::{Editor, Group};
    use std::cell::RefCell;
    const ID: &str = "solstice";
    #[derive(Default)]
    struct Gui { open: bool, closing: bool, drag: Option<()>, text_edit: Option<()>, text_key_consumed: bool }
    #[derive(Default)]
    struct State {
        hud_edit: Editor, hud_edit_dimension: Option<String>,
        gui: Gui, binding: Option<usize>,
    }
    thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }
    fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
        STATE.with(|state| f(&mut state.borrow_mut()))
    }
    struct Ui { hide_gui: bool }
    struct Dim(&'static str);
    impl Dim { fn id(&self) -> &str { self.0 } }
    struct Config { width: u32, height: u32 }
    struct Renderer { config: Config }
    // Scale-one host viewport double. Production calls the real HudBatch::new.
    struct HudBatch { width: f32, height: f32 }
    impl HudBatch {
        fn new(size: [u32; 2]) -> Self { Self { width: size[0] as f32, height: size[1] as f32 } }
    }
    pub(super) struct Game<'a> {
        window: bool, enabled: bool, playing: bool, player: Option<()>, focused: bool,
        ui: Ui, control: Option<()>, dim: Dim, renderer: Option<Renderer>, cursor: (f32, f32),
        captures: usize, saves: usize, marker: std::marker::PhantomData<&'a ()>,
    }
    impl Game<'_> {
        fn native_window(&self) -> bool { self.window }
        fn native_on(&self, _: &str) -> bool { self.enabled }
        fn native_playing(&self) -> bool { self.playing }
        fn capture(&mut self, captured: bool) { assert!(captured); self.captures += 1; }
    }
    fn save(game: &mut Game<'_>) { game.saves += 1; }
    mod solstice {
        pub(super) fn close_gui(game: &mut super::Game<'_>) { super::close_gui(game); }
    }
    fn game() -> Game<'static> {
        Game {
            window: true, enabled: true, playing: true, player: Some(()), focused: true,
            ui: Ui { hide_gui: false }, control: None, dim: Dim("overworld"),
            renderer: Some(Renderer { config: Config { width: 400, height: 300 } }),
            cursor: (200.0, 96.0), captures: 0, saves: 0, marker: std::marker::PhantomData,
        }
    }
    fn start_drag() {
        with(|state| {
            *state = State::default();
            state.hud_edit.begin();
            state.hud_edit.frame([400.0, 300.0], [0.0; 2], false);
            state.hud_edit.place(Group::Info, [150.0, 22.0], [2.0, 16.0]);
            state.hud_edit.press([80.0, 36.0]);
            state.hud_edit_dimension = Some("overworld".into());
            state.gui.open = true;
        });
    }
''' + close_gui + release + r'''
    #[test]
    fn closing_only_recaptures_a_focused_enabled_client_in_a_live_world() {
        for flags in 0..32 {
            with(|state| {
                *state = State::default();
                state.hud_edit.begin();
                state.hud_edit_dimension = Some("overworld".into());
                state.gui = Gui { open: true, closing: true, drag: Some(()), text_edit: Some(()), text_key_consumed: true };
                state.binding = Some(0);
            });
            let mut game = Game {
                window: flags & 1 != 0, enabled: flags & 2 != 0,
                playing: flags & 4 != 0, player: (flags & 8 != 0).then_some(()),
                focused: flags & 16 != 0, ..game()
            };
            close_gui(&mut game);
            assert_eq!(game.captures, usize::from(flags == 31), "context flags {flags}");
            assert_eq!(game.saves, 1);
            with(|state| {
                assert!(!state.hud_edit.active());
                assert!(state.hud_edit_dimension.is_none());
                assert!(!state.gui.open && !state.gui.closing);
                assert!(state.gui.drag.is_none() && state.binding.is_none());
                assert!(state.gui.text_edit.is_none() && !state.gui.text_key_consumed);
            });
        }
    }
    #[test]
    fn actual_release_adapter_finishes_a_fast_drag_once_at_the_release_cursor() {
        start_drag();
        let mut game = game();
        release(&mut game, "key.mouse.right");
        with(|state| assert!(state.hud_edit.position(Group::Info).is_none()));
        release(&mut game, "key.mouse.left");
        with(|state| assert_eq!(state.hud_edit.place(Group::Info, [150.0, 22.0], [2.0, 16.0]), [122.0, 76.0]));
        game.cursor = (390.0, 280.0);
        release(&mut game, "key.mouse.left");
        with(|state| {
            state.hud_edit.frame([400.0, 300.0], [390.0, 280.0], false);
            assert_eq!(state.hud_edit.place(Group::Info, [150.0, 22.0], [2.0, 16.0]), [122.0, 76.0]);
            assert!(state.hud_edit.active());
        });
        assert_eq!(game.saves, 0); // Release changes the draft, never Apply/persistence.
        assert_eq!(game.captures, 0);
    }
    #[test]
    fn actual_release_adapter_cancels_on_resize_before_the_next_hud_frame() {
        start_drag();
        let mut game = game();
        game.renderer.as_mut().unwrap().config.width = 500;
        release(&mut game, "key.mouse.left");
        with(|state| assert!(state.hud_edit.position(Group::Info).is_none()));
        assert_eq!(game.saves, 0);
    }
    #[test]
    fn actual_release_adapter_rejects_lost_lifecycle_contexts() {
        for gate in 0..8 {
            start_drag();
            let mut game = game();
            match gate {
                0 => game.window = false,
                1 => game.enabled = false,
                2 => game.playing = false,
                3 => game.player = None,
                4 => game.ui.hide_gui = true,
                5 => game.focused = false,
                6 => game.dim = Dim("nether"),
                _ => game.renderer = None,
            }
            release(&mut game, "key.mouse.left");
            with(|state| {
                assert!(!state.hud_edit.active());
                assert!(!state.gui.open);
            });
        }
    }
}
'''
    with tempfile.TemporaryDirectory(prefix="sunset-hud-tests-") as directory:
        directory = Path(directory)
        wrapper = directory / "tests.rs"
        wrapper.write_text(f'#[path = "{source.as_posix()}"] mod hud_layout;\n' + fixture, encoding="utf-8")
        binary = directory / ("hud-layout-tests.exe" if os.name == "nt" else "hud-layout-tests")
        subprocess.run([args.rustc, "--edition=2024", "--test", str(wrapper), "-o", str(binary)], check=True)
        subprocess.run([str(binary)], check=True)


if __name__ == "__main__":
    main()
