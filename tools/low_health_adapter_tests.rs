// SPDX-License-Identifier: MIT
// Copyright (c) 2026 PointOfPressure
// AI-assisted implementation: Codex (OpenAI).
// Includes the actual adapter with a minimal host test double. The pinned-host
// Cargo check separately verifies the real API. No game process or actions.
#![allow(dead_code)]
extern crate self as horizon_client;

use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[path = "../mixins/solstice/low_health_model.rs"]
mod solstice_low_health_model;
#[path = "../mixins/solstice/low_health.rs"]
mod solstice_low_health;

pub mod extras {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Corner { TopLeft, TopRight, BottomLeft, BottomRight }
}
pub mod extras_hud {
    use super::extras::Corner;
    #[derive(Default)]
    pub struct Corners { pub placements: Vec<Corner> }
    impl Corners {
        pub fn place(&mut self, corner: Corner, _width: f32, _height: f32) -> (f32, f32) {
            self.placements.push(corner);
            (2.0, 2.0)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen { Playing, Dead }
struct Ui { hide_gui: bool }
struct Health { hp: f32, absorption: f32, dead: bool }
impl Health {
    fn health(&self) -> f32 { self.hp }
    fn absorption(&self) -> f32 { self.absorption }
    fn dead_or_dying(&self) -> bool { self.hp <= 0.0 || self.dead }
}
struct Player { health: Health }
struct Game<'a> {
    window: bool,
    native: bool,
    title: Option<()>,
    player: Option<Player>,
    screen: Screen,
    ui: Ui,
    reserved: bool,
    marker: std::marker::PhantomData<&'a ()>,
}
impl Game<'_> {
    fn native_window(&self) -> bool { self.window }
    fn native_on(&self, _id: &str) -> bool { self.native }
    fn native_player(&self) -> Option<()> { self.player.as_ref().map(|_| ()) }
    fn native_corner_shown(&self, _corner: extras::Corner) -> bool { !self.reserved }
    fn native_text_width(&self, text: &str) -> f32 { text.len() as f32 * 6.0 }
}

static LOADS: AtomicUsize = AtomicUsize::new(0);
static PACKET_FLAG: AtomicBool = AtomicBool::new(false);
static TEST_LOCK: Mutex<()> = Mutex::new(());

mod solstice {
    use super::*;
    pub(super) const ID: &str = "solstice";
    #[derive(Clone)]
    pub(super) struct Module {
        pub name: &'static str,
        pub on: bool,
        pub threshold: f64,
        pub hysteresis: f64,
        pub corner: &'static str,
    }
    impl Module {
        pub(super) fn n(&self, name: &str) -> f64 {
            match name { "Threshold HP" => self.threshold, "Hysteresis HP" => self.hysteresis, _ => 0.0 }
        }
        pub(super) fn p(&self, _name: &str) -> &'static str { self.corner }
    }
    pub(super) enum Shape { Fill([f32; 4], u32), Text(String, [f32; 2], u32) }
    pub(super) struct State {
        pub modules: Vec<Module>,
        pub low_health: solstice_low_health_model::Warning,
    }
    impl Default for State {
        fn default() -> Self {
            Self {
                modules: vec![Module { name: "Low Health", on: true, threshold: 6.0, hysteresis: 2.0, corner: "Bottom right" }],
                low_health: Default::default(),
            }
        }
    }
    pub(super) fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
        static STATE: OnceLock<Mutex<State>> = OnceLock::new();
        f(&mut STATE.get_or_init(|| Mutex::new(State::default())).lock().unwrap())
    }
    pub(super) fn ensure_loaded(_game: &Game<'_>) {
        LOADS.fetch_add(1, Ordering::SeqCst);
        // Represents sync_flags processing enabled modules from saved config.
        PACKET_FLAG.store(true, Ordering::SeqCst);
    }
}

fn reset() {
    solstice::with(|state| *state = Default::default());
    LOADS.store(0, Ordering::SeqCst);
    PACKET_FLAG.store(false, Ordering::SeqCst);
}
fn game(hp: f32) -> Game<'static> {
    Game {
        window: true, native: true, title: None, screen: Screen::Playing,
        ui: Ui { hide_gui: false }, reserved: false,
        player: Some(Player { health: Health { hp, absorption: 20.0, dead: false } }),
        marker: Default::default(),
    }
}
fn draw(game: &Game<'_>, corners: &mut extras_hud::Corners) -> Vec<solstice::Shape> {
    let modules = solstice::with(|s| s.modules.clone());
    let mut shapes = Vec::new();
    solstice_low_health::draw(game, &modules, corners, &mut shapes);
    shapes
}

#[test]
fn disabled_native_never_loads_saved_module_flags_and_resets_only_warning() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let config = Some(solstice_low_health_model::Config { threshold: 6.0, hysteresis: 2.0 });
    solstice::with(|state| { state.low_health.update(config, Some(5.0)); });
    let mut game = game(5.0);
    game.native = false;
    assert_eq!(solstice_low_health::update(&game), None);
    assert_eq!(LOADS.load(Ordering::SeqCst), 0);
    assert!(!PACKET_FLAG.load(Ordering::SeqCst));
    assert!(solstice::with(|state| state.modules[0].on));
    assert_eq!(solstice::with(|state| state.low_health.update(config, Some(7.0))), None);
    assert!(draw(&game, &mut Default::default()).is_empty());
    assert_eq!(LOADS.load(Ordering::SeqCst), 0);
}

#[test]
fn server_game_cannot_load_or_reset_the_window_latch() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let config = Some(solstice_low_health_model::Config { threshold: 6.0, hysteresis: 2.0 });
    solstice::with(|state| { state.low_health.update(config, Some(5.0)); });
    let mut game = game(20.0);
    game.window = false;
    assert_eq!(solstice_low_health::update(&game), None);
    assert_eq!(LOADS.load(Ordering::SeqCst), 0);
    assert!(solstice::with(|state| state.low_health.update(config, Some(7.0))).is_some());
}

#[test]
fn base_health_and_hidden_hud_updates_flow_through_actual_adapter() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let mut game = game(5.0); // 20 absorption HP must not mask 5 base HP.
    let mut corners = extras_hud::Corners::default();
    let shapes = draw(&game, &mut corners);
    assert_eq!(corners.placements, vec![extras::Corner::BottomRight]);
    assert_eq!(shapes.len(), 2);
    assert!(matches!(&shapes[1], solstice::Shape::Text(text, _, _) if text == "Low health: 5.0 HP (threshold 6)"));
    game.ui.hide_gui = true;
    assert!(draw(&game, &mut corners).is_empty());
    game.player.as_mut().unwrap().health.hp = 9.0;
    assert_eq!(solstice_low_health::update(&game), None);
    game.ui.hide_gui = false;
    game.player.as_mut().unwrap().health.hp = 7.0;
    assert!(draw(&game, &mut corners).is_empty());
    game.player.as_mut().unwrap().health.hp = 5.0;
    game.reserved = true;
    assert!(draw(&game, &mut corners).is_empty());
}

#[test]
fn death_lifecycle_and_disable_gates_clear_adapter_latch() {
    let _guard = TEST_LOCK.lock().unwrap();
    for gate in 0..6 {
        reset();
        let mut game = game(5.0);
        assert!(solstice_low_health::update(&game).is_some());
        match gate {
            0 => game.screen = Screen::Dead,
            1 => game.player.as_mut().unwrap().health.dead = true,
            2 => game.player = None,
            3 => game.title = Some(()),
            4 => solstice::with(|state| state.modules[0].on = false),
            _ => game.native = false,
        }
        assert_eq!(solstice_low_health::update(&game), None);
        let game = self::game(7.0);
        solstice::with(|state| state.modules[0].on = true);
        assert_eq!(solstice_low_health::update(&game), None);
    }
}
