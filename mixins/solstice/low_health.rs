// SPDX-License-Identifier: MIT
// Copyright (c) 2026 PointOfPressure
// AI-assisted implementation: Codex (OpenAI).

use super::{Game, Screen};
use super::solstice::{ID, Module, Shape, ensure_loaded, with};
use super::solstice_low_health_model::{Config, Reading, visible};
use horizon_client::{extras::Corner, extras_hud::Corners};

pub(super) fn update(game: &Game<'_>) -> Option<Reading> {
    // State belongs to the window. A server Game must not reset its latch.
    if !game.native_window() {
        return None;
    }
    // Loading settings can synchronize other modules' packet/input flags.
    // A disabled native mod must only clear this warning's transient latch.
    if !game.native_on(ID) {
        with(|state| state.low_health.update(None, None));
        return None;
    }
    ensure_loaded(game);
    let in_world = game.title.is_none()
        && game.screen != Screen::Dead && game.native_player().is_some();
    // Health::health is the actual local base HP. Deliberately do not add
    // Health::absorption: a temporary shield must not mask low base health.
    let health = if in_world {
        game.player.as_ref().filter(|p| !p.health.dead_or_dying())
            .map(|p| f64::from(p.health.health()))
    } else {
        None
    };
    with(|state| {
        let config = state.modules.iter().find(|m| m.name == "Low Health" && m.on).map(|m| Config {
            threshold: m.n("Threshold HP"),
            hysteresis: m.n("Hysteresis HP"),
        });
        state.low_health.update(config, health)
    })
}

pub(super) fn draw(game: &Game<'_>, modules: &[Module], corners: &mut Corners, out: &mut Vec<Shape>) {
    // Sample here too so a health/settings change after native_frame cannot
    // leave stale text on this rendered frame. Repeated samples are idempotent.
    let reading = update(game);
    let Some(module) = modules.iter().find(|m| m.name == "Low Health" && m.on) else { return };
    let corner = match module.p("Corner") {
        "Bottom left" => Corner::BottomLeft,
        "Top left" => Corner::TopLeft,
        "Top right" => Corner::TopRight,
        _ => Corner::BottomRight,
    };
    if !visible(reading.is_some(), !game.ui.hide_gui, game.native_corner_shown(corner)) {
        return;
    }
    let Some(reading) = reading else { return };
    let text = reading.text();
    let width = game.native_text_width(&text) + 4.0;
    let (x, y) = corners.place(corner, width, 12.0);
    out.push(Shape::Fill([x, y, width, 12.0], 0x78000000));
    out.push(Shape::Text(text, [x + 2.0, y + 2.0], 0xFF5555));
}
