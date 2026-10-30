// SPDX-License-Identifier: MIT
// Copyright (c) 2026 PointOfPressure
// AI-assisted implementation: Codex (OpenAI).

use super::Game;
use super::solstice::{ACCENT, Module, Shape, health_colour};
use super::solstice_armor_model::{Durability, Number, durability, visible};
use horizon_client::{extras::Corner, extras_hud::Corners, items::ItemStack};

fn read(game: &Game<'_>, stack: Option<&ItemStack>) -> Durability {
    let Some(stack) = stack else { return Durability::Empty };
    if stack.count == 0 || &*stack.item == "minecraft:air" {
        return Durability::Empty;
    }
    // stack_view serializes count/damage as signed ints. Reject overflow before
    // constructing it. No guessed material tables or item-name heuristics.
    if stack.count > i32::MAX as u32 || stack.damage > i32::MAX as u32 {
        return Durability::Unknown;
    }
    let models = &game.pack.item_renderer.models;
    let view = horizon_client::item_render::stack_view(stack);
    // Host API merges exported item prototypes with stack component patches,
    // including removals and custom max_damage/unbreakable. Missing prototypes
    // are unknown: absent data must not imply an indestructible item.
    let components = models.components(&view);
    let number = |key| match components.get(key) {
        None => Number::Missing,
        Some(v) => v.as_i64().map_or(Number::Invalid, Number::Value),
    };
    let unbreakable = match components.get("minecraft:unbreakable") {
        None => Some(false),
        Some(v) if v.is_object() => Some(true),
        Some(_) => None,
    };
    durability(
        false,
        models.prototypes.contains_key(&view.item) && game.items().get(&stack.item).is_some(),
        unbreakable,
        number("minecraft:max_damage"),
        number("minecraft:damage"),
    )
}

pub(super) fn draw(game: &Game<'_>, modules: &[Module], corners: &mut Corners, out: &mut Vec<Shape>) {
    let Some(module) = modules.iter().find(|m| m.name == "Armor Durability") else { return };
    let corner = match module.p("Corner") {
        "Bottom right" => Corner::BottomRight,
        "Top left" => Corner::TopLeft,
        "Top right" => Corner::TopRight,
        _ => Corner::BottomLeft,
    };
    if !visible(
        module.on,
        game.native_on(super::solstice::ID),
        game.native_window(),
        game.native_player().is_some() && game.title.is_none(),
        !game.ui.hide_gui,
        game.native_corner_shown(corner),
    ) {
        return;
    }
    // The host armor array is explicitly head, chest, legs, feet (slots 500-503).
    let rows: Vec<_> = ["Head", "Chest", "Legs", "Feet"].into_iter()
        .zip(&game.armor)
        .map(|(slot, stack)| {
            let state = read(game, stack.as_ref());
            let color = match state {
                Durability::Remaining { remaining, max } => health_colour(remaining as f32 / max as f32),
                Durability::Unknown => 0xFF5555,
                Durability::Empty | Durability::NotDamageable => 0xAAAAAA,
                Durability::Unbreakable => 0xFFFFFF,
            };
            (state.text(slot, module.p("Display")), color)
        }).collect();
    let title = "Armor durability";
    let width = rows.iter().map(|(text, _)| game.native_text_width(text))
        .fold(game.native_text_width(title), f32::max) + 4.0;
    let height = 52.0;
    let (x, y) = corners.place(corner, width, height);
    out.push(Shape::Fill([x, y, width, height], 0x78000000));
    out.push(Shape::Text(title.to_owned(), [x + 2.0, y + 2.0], ACCENT));
    for (i, (text, color)) in rows.into_iter().enumerate() {
        out.push(Shape::Text(text, [x + 2.0, y + 12.0 + i as f32 * 10.0], color));
    }
}
