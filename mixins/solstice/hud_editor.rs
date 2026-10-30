// SPDX-License-Identifier: MIT
// Original Sunset HUD editor, written with Codex (OpenAI), 2026.
use super::solstice_hud_layout::{self as layout, Action, Group, Layout};
use super::{
    Game,
    solstice::{self, ACCENT, ID, Shape, with},
};
use horizon_client::{extras::Corner, extras_hud::Corners, hud::HudBatch};
use std::io::Read;

const FILE: &str = "hud-layout-v1";

pub(super) fn load(game: &Game<'_>) {
    let result = (|| {
        let file = match std::fs::File::open(game.native_data_dir(ID).join(FILE)) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Layout::default()),
            Err(_) => return Err("Could not read HUD positions; using corners"),
        };
        let mut text = String::new();
        file.take(layout::MAX_FILE_BYTES as u64 + 1)
            .read_to_string(&mut text)
            .map_err(|_| "Could not read HUD positions; using corners")?;
        Layout::decode(&text)
    })();
    with(|s| match result {
        Ok(value) => s.hud_edit.saved = value,
        Err(error) => s.hud_edit.error = Some(error),
    });
}

// The existing native_frame hook also runs when F1 hides native_hud. Cancel
// rather than retaining pointer state across screens, focus loss or worlds.
pub(super) fn frame(game: &mut Game<'_>) {
    if !with(|s| s.hud_edit.active()) {
        return;
    }
    if !game.native_window()
        || !game.native_on(ID)
        || !game.native_playing()
        || game.player.is_none()
        || game.ui.hide_gui
        || (!game.focused && game.control.is_none())
        || with(|s| s.hud_edit_dimension.as_deref() != Some(game.dim.id()))
    {
        solstice::close_gui(game);
        return;
    }
    game.keys.clear();
    game.left = false;
    game.right = false;
    match with(|s| s.hud_edit.pending.take()) {
        Some(Action::Apply) => {
            let text = with(|s| s.hud_edit.draft().map(Layout::encode));
            let Some(text) = text else {
                return;
            };
            let dir = game.native_data_dir(ID);
            // Fixed file names only. Commit the in-memory draft only after an
            // atomic same-directory replacement succeeds; failures remain editable.
            let temporary = dir.join("hud-layout-v1.tmp");
            let result = std::fs::create_dir_all(&dir)
                .and_then(|()| std::fs::write(&temporary, text))
                .and_then(|()| std::fs::rename(&temporary, dir.join(FILE)));
            if result.is_err() {
                let _ = std::fs::remove_file(&temporary);
                with(|s| {
                    s.hud_edit.error = Some("Could not save HUD positions. Retry Apply or Cancel.")
                });
            } else {
                with(|s| {
                    s.hud_edit.commit();
                    s.hud_edit.error = None;
                });
                solstice::close_gui(game);
            }
        }
        Some(Action::Cancel) => solstice::close_gui(game),
        Some(Action::Reset) => with(|s| s.hud_edit.reset()),
        None => {}
    }
}

// Mouse-up reaches native_key through extras_mouse even between HUD frames.
pub(super) fn release(game: &mut Game<'_>, key: &str) {
    if key != "key.mouse.left" || !with(|s| s.hud_edit.active()) {
        return;
    }
    if !game.native_window()
        || !game.native_on(ID)
        || !game.native_playing()
        || game.player.is_none()
        || game.ui.hide_gui
        || (!game.focused && game.control.is_none())
        || with(|s| s.hud_edit_dimension.as_deref() != Some(game.dim.id()))
    {
        solstice::close_gui(game);
        return;
    }
    let Some(renderer) = game.renderer.as_ref() else {
        solstice::close_gui(game);
        return;
    };
    // A resize can arrive before another HUD frame. Use the host's current
    // GUI scale calculation instead of the previous rendered bounds' viewport.
    let batch = HudBatch::new([renderer.config.width, renderer.config.height]);
    let at = [game.cursor.0, game.cursor.1];
    with(|s| s.hud_edit.release([batch.width, batch.height], at));
}

pub(super) fn start_frame(game: &Game<'_>, view: [f32; 2]) {
    let at = [game.cursor.0, game.cursor.1];
    let left = game
        .physical_buttons
        .contains(&winit::event::MouseButton::Left);
    with(|s| s.hud_edit.frame(view, at, left));
}

pub(super) fn place(
    game: &Game<'_>,
    corners: &mut Corners,
    group: Group,
    default: Corner,
    size: [f32; 2],
) -> Option<[f32; 2]> {
    let position = with(|s| s.hud_edit.position(group));
    let corner = position.map_or(default, |p| match (p.right, p.bottom) {
        (false, false) => Corner::TopLeft,
        (true, false) => Corner::TopRight,
        (false, true) => Corner::BottomLeft,
        (true, true) => Corner::BottomRight,
    });
    if !game.native_corner_shown(corner) {
        return None;
    }
    let auto = if position.is_none() {
        let (x, y) = corners.place(default, size[0], size[1]);
        [x, y]
    } else {
        [0.0; 2]
    };
    Some(with(|s| s.hud_edit.place(group, size, auto)))
}

pub(super) fn draw(game: &Game<'_>, view: [f32; 2], out: &mut Vec<Shape>) {
    let (bounds, focus, error) = with(|s| (s.hud_edit.bounds, s.hud_edit.focus, s.hud_edit.error));
    let at = [game.cursor.0, game.cursor.1];
    for r in bounds.into_iter().flatten() {
        let [x, y, w, h] = r;
        let colour = if layout::contains(r, at) {
            0xFFFF_FFFF
        } else {
            0xFF00_0000 | ACCENT
        };
        for edge in [
            [x, y, w, 0.5],
            [x, y + h - 0.5, w, 0.5],
            [x, y, 0.5, h],
            [x + w - 0.5, y, 0.5, h],
        ] {
            out.push(Shape::Fill(edge, colour));
        }
    }
    let buttons = layout::buttons(view);
    let top = buttons[0].1[1];
    out.push(Shape::Fill(
        [
            0.0,
            (top - 28.0).max(0.0),
            view[0],
            view[1] - (top - 28.0).max(0.0),
        ],
        0xE000_0000,
    ));
    let lines = if view[0] >= 360.0 {
        [
            "HUD positions: drag visible gold boxes",
            "Tab / Shift+Tab selects a button; Enter confirms; Esc cancels",
        ]
    } else {
        [
            "HUD positions: drag gold boxes",
            "Tab: button  Enter: choose  Esc: cancel",
        ]
    };
    for (i, text) in lines.into_iter().enumerate() {
        out.push(Shape::Text(
            text.into(),
            [4.0, (top - 25.0 + i as f32 * 10.0).max(0.0)],
            if i == 0 { ACCENT } else { 0xFFFFFF },
        ));
    }
    for (i, (action, r)) in buttons.into_iter().enumerate() {
        let hover = layout::contains(r, at);
        out.push(Shape::Fill(
            r,
            if hover || i == focus {
                0xFF50_3A14
            } else {
                0xFF30_3030
            },
        ));
        if i == focus {
            out.push(Shape::Fill(
                [r[0], r[1] + r[3] - 1.0, r[2], 1.0],
                0xFF00_0000 | ACCENT,
            ));
        }
        let label = action.label();
        out.push(Shape::Text(
            label.into(),
            [
                r[0] + (r[2] - game.native_text_width(label)) / 2.0,
                r[1] + 3.0,
            ],
            0xFFFFFF,
        ));
    }
    if let Some(error) = error {
        out.push(Shape::Fill([0.0, 0.0, view[0], 12.0], 0xE000_0000));
        out.push(Shape::Text(error.into(), [4.0, 2.0], 0xFF7777));
    }
}
