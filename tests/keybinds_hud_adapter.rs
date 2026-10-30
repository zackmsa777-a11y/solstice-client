// SPDX-License-Identifier: MIT
// Copyright (c) 2026 PointOfPressure
// AI-assisted implementation: Codex (OpenAI).
// Host vocabulary is supplied here; adapter/model/key formatter are production.
use std::sync::{Mutex, OnceLock};

pub mod extras {
    #[derive(Clone, Copy)] pub enum Corner { TopLeft, TopRight, BottomLeft, BottomRight }
    impl Corner {
        pub fn top(self) -> bool { matches!(self, Self::TopLeft | Self::TopRight) }
        pub fn left(self) -> bool { matches!(self, Self::TopLeft | Self::BottomLeft) }
    }
}
pub mod extras_hud {
    use super::extras::Corner;
    pub struct Corners { pub view: [f32; 2], pub used: [f32; 4] }
    impl Corners {
        pub fn new(view: [f32; 2]) -> Self { Self { view, used: [0.0; 4] } }
        pub fn place(&mut self, corner: Corner, width: f32, height: f32) -> (f32, f32) {
            let index = corner as usize;
            let edge = self.used[index] + 2.0;
            self.used[index] += height + 2.0;
            (if corner.left() { 2.0 } else { self.view[0] - width - 2.0 },
             if corner.top() { edge } else { self.view[1] - edge - height })
        }
    }
}
struct Ui { hide_gui: bool }
struct Game<'a> {
    window: bool, enabled: bool, player: Option<()>, title: Option<()>, ui: Ui, corner_available: bool,
    marker: std::marker::PhantomData<&'a ()>,
}
impl Game<'_> {
    fn native_window(&self) -> bool { self.window }
    fn native_on(&self, _: &str) -> bool { self.enabled }
    fn native_corner_shown(&self, _: extras::Corner) -> bool { self.corner_available }
    fn native_text_width(&self, text: &str) -> f32 {
        // The actual font hook may access settings. Fail fast if the adapter
        // measures while it still holds the state mutex.
        solstice::with(|_| ());
        text.chars().count() as f32 * 6.0
    }
}
mod solstice {
    use super::*;
    pub(super) const ID: &str = "solstice";
    pub(super) const ACCENT: u32 = 0xFFB020;
    pub(super) enum Shape { Fill([f32; 4], u32), Text(String, [f32; 2], u32) }
    pub(super) struct Module {
        pub name: &'static str, pub key: Option<String>, pub on: bool,
        pub only_active: bool, pub max_rows: f64, pub corner: &'static str,
        pub settings_blob: Vec<u8>,
    }
    impl Clone for Module { fn clone(&self) -> Self { panic!("HUD must not clone a module/settings"); } }
    impl Module {
        pub(super) fn b(&self, _: &str) -> bool { self.only_active }
        pub(super) fn n(&self, _: &str) -> f64 { self.max_rows }
        pub(super) fn p(&self, _: &str) -> &'static str { self.corner }
    }
    pub(super) struct State { pub modules: Vec<Module> }
    pub(super) fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
        static STATE: OnceLock<Mutex<State>> = OnceLock::new();
        f(&mut STATE.get_or_init(|| Mutex::new(State { modules: vec![] })).try_lock().expect("state lock reentry"))
    }
    // PRODUCTION_KEY_LABEL
}
static SERIAL: Mutex<()> = Mutex::new(());
fn module(name: &'static str, key: Option<&str>, on: bool) -> solstice::Module {
    solstice::Module { name, key: key.map(str::to_owned), on, only_active: false, max_rows: 8.0, corner: "Bottom left", settings_blob: vec![5; 4096] }
}
fn reset() {
    solstice::with(|state| state.modules = vec![module("Keybinds HUD", None, true),
        module("Unknown", Some("vendor.device.99"), false), module("Zoom", Some("key.keyboard.z"), true)]);
}
fn game() -> Game<'static> {
    Game { window: true, enabled: true, player: Some(()), title: None, ui: Ui { hide_gui: false }, corner_available: true, marker: Default::default() }
}
fn snapshot() -> Vec<(&'static str, Option<String>, bool, usize)> {
    solstice::with(|state| state.modules.iter().map(|m| (m.name, m.key.clone(), m.on, m.settings_blob.len())).collect())
}
fn draw(game: &Game<'_>, corners: &mut extras_hud::Corners) -> Vec<solstice::Shape> {
    let mut out = Vec::new();
    let view = corners.view;
    adapter::draw(game, corners, view, &mut out);
    out
}

#[test]
fn actual_adapter_uses_normal_unknown_key_label_and_effective_status_without_mutation() {
    let _guard = SERIAL.lock().unwrap(); reset();
    let before = snapshot();
    let out = draw(&game(), &mut extras_hud::Corners::new([400.0, 300.0]));
    let text: Vec<&str> = out.iter().filter_map(|shape| match shape { solstice::Shape::Text(text, _, _) => Some(text.as_str()), _ => None }).collect();
    assert!(text.contains(&"Vendor Device 99"));
    assert!(text.contains(&"ON") && text.contains(&"OFF"));
    assert_eq!(snapshot(), before);
}

#[test]
fn every_visibility_gate_prevents_drawing_or_corner_reservation() {
    let _guard = SERIAL.lock().unwrap();
    for gate in 0..7 {
        reset(); let mut game = game();
        match gate {
            0 => game.window = false, 1 => game.enabled = false, 2 => game.player = None,
            3 => game.title = Some(()), 4 => game.ui.hide_gui = true, 5 => game.corner_available = false,
            _ => solstice::with(|state| state.modules[0].on = false),
        }
        let mut corners = extras_hud::Corners::new([400.0, 300.0]);
        assert!(draw(&game, &mut corners).is_empty());
        assert_eq!(corners.used, [0.0; 4]);
    }
}

#[test]
fn tiny_or_exhausted_corners_never_emit_out_of_bounds_text() {
    let _guard = SERIAL.lock().unwrap(); reset();
    for view in [[50.0, 100.0], [400.0, 20.0], [400.0, 30.0], [90.0, 40.0]] {
        let out = draw(&game(), &mut extras_hud::Corners::new(view));
        for shape in out {
            match shape {
                solstice::Shape::Fill(rect, _) => assert!(crate::solstice_keybinds_model::inside([rect[0], rect[1]], [rect[2], rect[3]], view)),
                solstice::Shape::Text(text, at, _) => {
                    assert!(at[0] >= 2.0 && at[1] >= 2.0);
                    assert!(at[0] + game().native_text_width(&text) <= view[0] - 2.0);
                    assert!(at[1] + 8.0 <= view[1] - 2.0);
                }
            }
        }
    }
    let mut corners = extras_hud::Corners::new([400.0, 100.0]);
    corners.used[extras::Corner::BottomLeft as usize] = 90.0;
    assert!(draw(&game(), &mut corners).is_empty());
}

#[test]
fn active_filter_and_maximum_rows_use_live_facts_and_exact_overflow() {
    let _guard = SERIAL.lock().unwrap(); reset();
    solstice::with(|state| { state.modules[0].only_active = true; state.modules[0].max_rows = 1.0; state.modules.push(module("ESP", Some("key.keyboard.e"), true)); });
    let out = draw(&game(), &mut extras_hud::Corners::new([400.0, 300.0]));
    let text: Vec<&str> = out.iter().filter_map(|shape| match shape { solstice::Shape::Text(text, _, _) => Some(text.as_str()), _ => None }).collect();
    assert!(!text.contains(&"Unknown") && !text.contains(&"OFF"));
    assert!(text.contains(&"Zoom") && text.contains(&"+1 more"));
}
