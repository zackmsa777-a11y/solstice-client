// SPDX-License-Identifier: MIT
// Original HUD position model, written with Codex (OpenAI), 2026.
// Host-free: `python tools/test_hud_layout.py` exercises the model.

pub(super) const GROUPS: usize = 3;
pub(super) const MAX_FILE_BYTES: usize = 1024;
const MAX_OFFSET: f32 = 32768.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Group {
    Watermark,
    Modules,
    Info,
}

impl Group {
    pub(super) const ALL: [Self; GROUPS] = [Self::Watermark, Self::Modules, Self::Info];
    fn index(self) -> usize {
        self as usize
    }
    fn key(self) -> &'static str {
        match self {
            Self::Watermark => "watermark",
            Self::Modules => "modules",
            Self::Info => "info",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Position {
    pub(super) right: bool,
    pub(super) bottom: bool,
    x: f32,
    y: f32,
}

fn extent(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, MAX_OFFSET)
    } else {
        0.0
    }
}

impl Position {
    fn from_origin(at: [f32; 2], size: [f32; 2], view: [f32; 2]) -> Self {
        let room = [
            (extent(view[0]) - extent(size[0])).max(0.0),
            (extent(view[1]) - extent(size[1])).max(0.0),
        ];
        let x = extent(at[0]).min(room[0]);
        let y = extent(at[1]).min(room[1]);
        let (right, bottom) = (x > room[0] / 2.0, y > room[1] / 2.0);
        Self {
            right,
            bottom,
            x: if right { room[0] - x } else { x },
            y: if bottom { room[1] - y } else { y },
        }
    }
    fn origin(self, size: [f32; 2], view: [f32; 2]) -> [f32; 2] {
        let room = [
            (extent(view[0]) - extent(size[0])).max(0.0),
            (extent(view[1]) - extent(size[1])).max(0.0),
        ];
        [
            if self.right { room[0] - self.x } else { self.x }.clamp(0.0, room[0]),
            if self.bottom {
                room[1] - self.y
            } else {
                self.y
            }
            .clamp(0.0, room[1]),
        ]
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Layout {
    positions: [Option<Position>; GROUPS],
}

impl Layout {
    pub(super) fn encode(&self) -> String {
        let mut out = String::from("sunset-hud-layout=1\n");
        for group in Group::ALL {
            let value = match self.positions[group.index()] {
                None => "auto".to_owned(),
                Some(p) => format!(
                    "{},{},{},{}",
                    if p.right { "right" } else { "left" },
                    if p.bottom { "bottom" } else { "top" },
                    p.x,
                    p.y
                ),
            };
            out.push_str(&format!("{}={value}\n", group.key()));
        }
        out
    }
    // An unsupported, partial, duplicate or malformed file is rejected in full.
    pub(super) fn decode(text: &str) -> Result<Self, &'static str> {
        if text.len() > MAX_FILE_BYTES {
            return Err("HUD layout file is too large");
        }
        let mut lines = text.lines();
        if lines.next() != Some("sunset-hud-layout=1") {
            return Err("Unsupported HUD layout schema");
        }
        let mut out = Self::default();
        let mut seen = [false; GROUPS];
        for line in lines {
            let Some((key, value)) = line.split_once('=') else {
                return Err("Invalid HUD layout entry");
            };
            let Some(group) = Group::ALL.into_iter().find(|g| g.key() == key) else {
                return Err("Unknown HUD layout group");
            };
            let index = group.index();
            if std::mem::replace(&mut seen[index], true) {
                return Err("Duplicate HUD layout group");
            }
            if value == "auto" {
                continue;
            }
            let parts: Vec<_> = value.split(',').collect();
            let [horizontal, vertical, x, y] = parts.as_slice() else {
                return Err("Invalid HUD layout position");
            };
            let right = match *horizontal {
                "left" => false,
                "right" => true,
                _ => return Err("Invalid HUD horizontal anchor"),
            };
            let bottom = match *vertical {
                "top" => false,
                "bottom" => true,
                _ => return Err("Invalid HUD vertical anchor"),
            };
            let offset = |s: &str| {
                s.parse::<f32>()
                    .ok()
                    .filter(|n| n.is_finite() && (0.0..=MAX_OFFSET).contains(n))
            };
            let (Some(x), Some(y)) = (offset(x), offset(y)) else {
                return Err("Invalid HUD layout offset");
            };
            out.positions[index] = Some(Position {
                right,
                bottom,
                x,
                y,
            });
        }
        if !seen.into_iter().all(|v| v) {
            return Err("Incomplete HUD layout file");
        }
        Ok(out)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Apply,
    Cancel,
    Reset,
}

impl Action {
    pub(super) const ALL: [Self; 3] = [Self::Apply, Self::Cancel, Self::Reset];
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Apply => "Apply",
            Self::Cancel => "Cancel",
            Self::Reset => "Reset",
        }
    }
}

pub(super) fn contains(r: [f32; 4], at: [f32; 2]) -> bool {
    at[0].is_finite()
        && at[1].is_finite()
        && at[0] >= r[0]
        && at[0] < r[0] + r[2]
        && at[1] >= r[1]
        && at[1] < r[1] + r[3]
}

// The same rectangles are drawn and hit-tested. Narrow views use a vertical toolbar.
pub(super) fn buttons(view: [f32; 2]) -> [(Action, [f32; 4]); 3] {
    let wide = view[0] >= 190.0;
    let width = if wide {
        58.0
    } else {
        (view[0] - 4.0).clamp(1.0, 110.0)
    };
    let all_width = if wide { width * 3.0 + 8.0 } else { width };
    let x = ((view[0] - all_width) / 2.0).max(0.0);
    let y = (view[1] - if wide { 18.0 } else { 54.0 }).max(0.0);
    std::array::from_fn(|i| {
        (
            Action::ALL[i],
            [
                x + if wide { i as f32 * (width + 4.0) } else { 0.0 },
                y + if wide { 0.0 } else { i as f32 * 17.0 },
                width,
                14.0,
            ],
        )
    })
}

#[derive(Default)]
pub(super) struct Editor {
    pub(super) saved: Layout,
    draft: Option<Layout>,
    // Measured content bounds from the last rendered frame, never synthetic panel bounds.
    pub(super) bounds: [Option<[f32; 4]>; GROUPS],
    drag: Option<(Group, [f32; 2])>,
    view: [f32; 2],
    pub(super) pending: Option<Action>,
    pub(super) focus: usize,
    pub(super) error: Option<&'static str>,
}

impl Editor {
    pub(super) fn active(&self) -> bool {
        self.draft.is_some()
    }
    pub(super) fn begin(&mut self) {
        self.cancel();
        self.draft = Some(self.saved.clone());
        self.focus = 0;
    }
    pub(super) fn cancel(&mut self) {
        self.draft = None;
        self.drag = None;
        self.bounds = [None; GROUPS];
        self.pending = None;
    }
    pub(super) fn reset(&mut self) {
        if self.active() {
            self.draft = Some(Layout::default());
        }
        self.drag = None;
        self.bounds = [None; GROUPS];
        self.error = None;
    }
    pub(super) fn draft(&self) -> Option<&Layout> {
        self.draft.as_ref()
    }
    pub(super) fn commit(&mut self) {
        if let Some(draft) = self.draft.take() {
            self.saved = draft;
        }
        self.cancel();
    }
    pub(super) fn position(&self, group: Group) -> Option<Position> {
        self.draft.as_ref().unwrap_or(&self.saved).positions[group.index()]
    }
    pub(super) fn frame(&mut self, view: [f32; 2], at: [f32; 2], left: bool) {
        let view = [extent(view[0]), extent(view[1])];
        // A resize invalidates the old hit boxes and pointer-to-group offset.
        if self.view != view {
            self.drag = None;
        }
        self.view = view;
        if !left {
            self.drag = None;
        }
        self.move_drag(at);
        self.bounds = [None; GROUPS];
    }
    fn move_drag(&mut self, at: [f32; 2]) {
        if let Some((group, offset)) = self.drag
            && let Some(r) = self.bounds[group.index()]
            && at.into_iter().all(f32::is_finite)
            && let Some(draft) = self.draft.as_mut()
        {
            draft.positions[group.index()] = Some(Position::from_origin(
                [at[0] - offset[0], at[1] - offset[1]],
                [r[2], r[3]],
                self.view,
            ));
        }
    }
    // Only a real release callback supplies its last observed cursor. A later
    // unheld frame must not be mistaken for that release point.
    pub(super) fn release(&mut self, view: [f32; 2], at: [f32; 2]) {
        let view = [extent(view[0]), extent(view[1])];
        if self.view != view {
            self.drag = None;
            self.bounds = [None; GROUPS];
            self.view = view;
            return;
        }
        self.move_drag(at);
        self.drag = None;
    }
    pub(super) fn place(&mut self, group: Group, size: [f32; 2], auto: [f32; 2]) -> [f32; 2] {
        let at = self
            .position(group)
            .map_or(auto, |p| p.origin(size, self.view));
        if self.active() {
            self.bounds[group.index()] = Some([at[0], at[1], size[0], size[1]]);
        }
        at
    }
    pub(super) fn press(&mut self, at: [f32; 2]) {
        if !self.active() || self.pending.is_some() {
            return;
        }
        if let Some((action, _)) = buttons(self.view)
            .into_iter()
            .find(|(_, r)| contains(*r, at))
        {
            self.drag = None;
            self.pending = Some(action);
            return;
        }
        // Reverse drawing order makes the topmost overlapping group draggable.
        self.drag = Group::ALL.into_iter().rev().find_map(|group| {
            let r = self.bounds[group.index()]?;
            contains(r, at).then_some((group, [at[0] - r[0], at[1] - r[1]]))
        });
    }
    pub(super) fn key(&mut self, name: &str, back: bool, menu: bool) {
        if !self.active() {
            return;
        }
        match name {
            "key.keyboard.escape" => self.pending = Some(Action::Cancel),
            "key.keyboard.tab" => self.focus = (self.focus + if back { 2 } else { 1 }) % 3,
            "key.keyboard.enter" | "key.keyboard.keypad.enter" => {
                self.pending = Some(Action::ALL[self.focus])
            }
            _ if menu => self.pending = Some(Action::Cancel),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn begin() -> Editor {
        let mut e = Editor::default();
        e.begin();
        e.frame([400.0, 300.0], [0.0; 2], false);
        e
    }
    #[test]
    fn default_keeps_host_reservations_exactly() {
        let mut e = begin();
        assert_eq!(
            e.place(Group::Watermark, [80.0, 12.0], [2.0, 26.0]),
            [2.0, 26.0]
        );
        assert_eq!(
            e.place(Group::Info, [180.0, 22.0], [2.0, 40.0]),
            [2.0, 40.0]
        );
        assert!(e.position(Group::Info).is_none());
    }
    #[test]
    fn drag_uses_measured_box_and_preserves_grab_offset() {
        let mut e = begin();
        e.place(Group::Info, [150.0, 22.0], [2.0, 16.0]);
        e.press([80.0, 36.0]); // near the far end of the second text row
        e.frame([400.0, 300.0], [200.0, 96.0], true);
        assert_eq!(
            e.place(Group::Info, [150.0, 22.0], [2.0, 16.0]),
            [122.0, 76.0]
        );
        assert!(e.saved.positions[2].is_none());
        e.cancel();
        assert!(e.position(Group::Info).is_none());
        assert!(e.drag.is_none());
    }
    #[test]
    fn manual_positions_clamp_then_keep_nearest_edge_on_resize() {
        let p = Position::from_origin([900.0, 900.0], [80.0, 20.0], [400.0, 300.0]);
        assert!(p.right && p.bottom);
        assert_eq!(p.origin([80.0, 20.0], [500.0, 400.0]), [420.0, 380.0]);
        assert_eq!(p.origin([80.0, 20.0], [40.0, 10.0]), [0.0, 0.0]);
        let p = Position::from_origin([302.0, 210.0], [80.0, 20.0], [400.0, 300.0]);
        assert_eq!(p.origin([100.0, 30.0], [500.0, 400.0]), [382.0, 300.0]);
    }
    #[test]
    fn release_resize_reset_and_cancel_do_not_leak_drags() {
        let mut e = begin();
        e.place(Group::Watermark, [80.0, 12.0], [2.0, 2.0]);
        e.press([8.0, 8.0]);
        e.frame([500.0, 300.0], [200.0, 200.0], true);
        assert!(e.position(Group::Watermark).is_none()); // resize cancels the stale drag
        e.place(Group::Watermark, [80.0, 12.0], [2.0, 2.0]);
        e.press([8.0, 8.0]);
        e.frame([500.0, 300.0], [200.0, 200.0], false);
        assert!(e.position(Group::Watermark).is_none());
        e.reset();
        assert!(e.drag.is_none());
        e.cancel();
        e.begin();
        assert!(e.bounds.iter().all(Option::is_none));
    }
    #[test]
    fn fast_release_commits_last_pointer_once_without_an_intermediate_frame() {
        let mut e = begin();
        e.place(Group::Info, [150.0, 22.0], [2.0, 16.0]);
        e.press([80.0, 36.0]);
        e.release([400.0, 300.0], [200.0, 96.0]);
        assert_eq!(e.place(Group::Info, [150.0, 22.0], [2.0, 16.0]), [122.0, 76.0]);
        e.release([400.0, 300.0], [350.0, 250.0]);
        e.frame([400.0, 300.0], [390.0, 280.0], false);
        assert_eq!(e.place(Group::Info, [150.0, 22.0], [2.0, 16.0]), [122.0, 76.0]);
        assert!(e.saved.positions[Group::Info.index()].is_none());
    }
    #[test]
    fn resize_before_release_cancels_the_old_grab_offset() {
        let mut e = begin();
        e.place(Group::Info, [150.0, 22.0], [2.0, 16.0]);
        e.press([80.0, 36.0]);
        e.release([500.0, 300.0], [200.0, 96.0]);
        assert!(e.position(Group::Info).is_none());
        assert!(e.drag.is_none());
        e.frame([500.0, 300.0], [300.0, 200.0], false);
        assert!(e.position(Group::Info).is_none());
    }
    #[test]
    fn apply_is_explicit_reset_is_draft_only() {
        let mut e = begin();
        e.place(Group::Modules, [80.0, 40.0], [318.0, 2.0]);
        e.press([320.0, 4.0]);
        e.frame([400.0, 300.0], [250.0, 90.0], true);
        let text = e.draft().unwrap().encode();
        assert_eq!(Layout::decode(&text).unwrap(), *e.draft().unwrap());
        e.commit();
        let saved = e.saved.clone();
        assert!(e.position(Group::Modules).is_some());
        e.begin();
        e.reset();
        e.cancel();
        assert_eq!(e.saved, saved);
        e.begin();
        e.reset();
        e.commit();
        assert_eq!(e.saved, Layout::default());
    }
    #[test]
    fn toolbar_wins_overlap_and_keyboard_never_changes_modules() {
        let mut e = begin();
        assert_eq!(Action::Apply.label(), "Apply");
        let (_, r) = buttons([400.0, 300.0])[0];
        e.place(Group::Info, [300.0, 30.0], [r[0], r[1]]);
        e.press([r[0] + 2.0, r[1] + 2.0]);
        assert_eq!(e.pending, Some(Action::Apply));
        assert!(e.drag.is_none());
        e.pending = None;
        e.key("key.keyboard.a", false, false);
        assert_eq!(e.pending, None);
        e.key("key.keyboard.tab", false, false);
        e.key("key.keyboard.enter", false, false);
        assert_eq!(e.pending, Some(Action::Cancel));
        e.key("key.keyboard.escape", false, false);
        assert_eq!(e.pending, Some(Action::Cancel));
    }
    #[test]
    fn topmost_measured_group_wins_and_invalid_input_is_ignored() {
        let mut e = begin();
        for g in Group::ALL {
            e.place(g, [100.0, 40.0], [2.0, 2.0]);
        }
        e.press([80.0, 30.0]);
        assert_eq!(e.drag.unwrap().0, Group::Info);
        e.frame([400.0, 300.0], [f32::NAN, 2.0], true);
        assert!(e.position(Group::Info).is_none());
        e.press([f32::NAN, 2.0]);
        assert!(e.drag.is_none());
    }
    #[test]
    fn strict_schema_rejects_bad_or_unbounded_positions() {
        let valid = Layout::default().encode();
        assert_eq!(Layout::decode(&valid).unwrap(), Layout::default());
        for value in [
            "left,top,NaN,2",
            "right,bottom,inf,0",
            "left,top,-1,2",
            "left,top,32769,2",
            "middle,top,0,0",
            "left,top,1",
            "left,top,1,2,3",
        ] {
            assert!(
                Layout::decode(&valid.replace("watermark=auto", &format!("watermark={value}")))
                    .is_err(),
                "{value}"
            );
        }
        assert!(Layout::decode(&valid.replace("layout=1", "layout=2")).is_err());
        assert!(Layout::decode(&valid.replace("info=auto\n", "")).is_err());
        assert!(Layout::decode(&(valid.clone() + "info=auto\n")).is_err());
        assert!(Layout::decode(&(valid.clone() + "radar=auto\n")).is_err());
        assert!(Layout::decode(&"x".repeat(MAX_FILE_BYTES + 1)).is_err());
    }
}
