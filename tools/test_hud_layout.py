#!/usr/bin/env python3
"""Test the production HUD model and extracted close_gui lifecycle guard."""
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
    # Compile the actual runtime body, not a duplicate of its capture condition.
    fixture = r'''
mod close_gui_tests {
    use super::hud_layout::Editor;
    use std::cell::RefCell;
    const ID: &str = "solstice";
    #[derive(Default)]
    struct Gui { open: bool, closing: bool, drag: Option<()> }
    #[derive(Default)]
    struct State {
        hud_edit: Editor, hud_edit_dimension: Option<String>,
        gui: Gui, binding: Option<usize>,
    }
    thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }
    fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
        STATE.with(|state| f(&mut state.borrow_mut()))
    }
    pub(super) struct Game<'a> {
        window: bool, enabled: bool, playing: bool, player: Option<()>, focused: bool,
        captures: usize, saves: usize, marker: std::marker::PhantomData<&'a ()>,
    }
    impl Game<'_> {
        fn native_window(&self) -> bool { self.window }
        fn native_on(&self, _: &str) -> bool { self.enabled }
        fn native_playing(&self) -> bool { self.playing }
        fn capture(&mut self, captured: bool) { assert!(captured); self.captures += 1; }
    }
    fn save(game: &mut Game<'_>) { game.saves += 1; }
''' + close_gui + r'''
    #[test]
    fn closing_only_recaptures_a_focused_enabled_client_in_a_live_world() {
        for flags in 0..32 {
            with(|state| {
                *state = State::default();
                state.hud_edit.begin();
                state.hud_edit_dimension = Some("overworld".into());
                state.gui = Gui { open: true, closing: true, drag: Some(()) };
                state.binding = Some(0);
            });
            let mut game = Game {
                window: flags & 1 != 0, enabled: flags & 2 != 0,
                playing: flags & 4 != 0, player: (flags & 8 != 0).then_some(()),
                focused: flags & 16 != 0, captures: 0, saves: 0,
                marker: std::marker::PhantomData,
            };
            close_gui(&mut game);
            assert_eq!(game.captures, usize::from(flags == 31), "context flags {flags}");
            assert_eq!(game.saves, 1);
            with(|state| {
                assert!(!state.hud_edit.active());
                assert!(state.hud_edit_dimension.is_none());
                assert!(!state.gui.open && !state.gui.closing);
                assert!(state.gui.drag.is_none() && state.binding.is_none());
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
