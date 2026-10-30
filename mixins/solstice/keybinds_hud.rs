// SPDX-License-Identifier: MIT
// Copyright (c) 2026 PointOfPressure
// AI-assisted implementation: Codex (OpenAI).

use super::Game;
use super::solstice::{ACCENT, ID, Shape, key_label, with};
use super::solstice_keybinds_model::{Fact, Ink, available_height, inside, layout, row_limit, select};
use horizon_client::{extras::Corner, extras_hud::Corners};

pub(super) fn draw(game: &Game<'_>, corners: &mut Corners, view: [f32; 2], out: &mut Vec<Shape>) {
    if !game.native_window() || !game.native_on(ID) || game.player.is_none()
        || game.title.is_some() || game.ui.hide_gui
        || !view.into_iter().all(|n| n.is_finite() && n > 0.0)
    {
        return;
    }
    let config = with(|state| {
        state.modules.iter().find(|m| m.name == "Keybinds HUD" && m.on)
            .map(|m| (m.b("Only active"), row_limit(m.n("Max rows")), m.p("Corner")))
    });
    let Some((only_active, limit, corner)) = config else { return };
    let corner = match corner {
        "Bottom right" => Corner::BottomRight,
        "Top left" => Corner::TopLeft,
        "Top right" => Corner::TopRight,
        _ => Corner::BottomLeft,
    };
    if !game.native_corner_shown(corner) { return; }
    // Copy only selected row facts. No Module/settings clones or state writes.
    let mut selected = with(|state| select(state.modules.iter().map(|m| Fact {
        name: m.name, key: m.key.as_deref(), on: m.on,
    }), only_active, limit));
    // The normal helper also picks up mouse labels when combined with binds.
    // Measure outside the state lock: the native font hook may read settings.
    for row in &mut selected.rows {
        row.key = key_label(Some(&row.key));
        if row.key.is_empty() { row.key = "(empty)".into(); }
    }
    // Corners exposes allocation, not remaining space. This zero-height anchor
    // claims only its normal 2px gap; then size the real box to available space.
    let (_, anchor) = corners.place(corner, 0.0, 0.0);
    let Some(room) = available_height(view[1], anchor, corner.top()) else { return };
    let Some(plan) = layout(&selected, only_active, view[0], room, |text| game.native_text_width(text)) else { return };
    let (x, y) = corners.place(corner, plan.width, plan.height);
    if !inside([x, y], [plan.width, plan.height], view) { return; }
    out.push(Shape::Fill([x, y, plan.width, plan.height], 0x78000000));
    for text in plan.text {
        let color = match text.ink {
            Ink::Title => ACCENT,
            Ink::Name => 0xFFFFFF,
            Ink::On => 0x55FF55,
            Ink::Key | Ink::Off | Ink::Footer => 0xAAAAAA,
        };
        out.push(Shape::Text(text.text, [x + text.at[0], y + text.at[1]], color));
    }
}
