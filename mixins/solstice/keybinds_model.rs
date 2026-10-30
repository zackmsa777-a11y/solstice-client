// SPDX-License-Identifier: MIT
// Copyright (c) 2026 PointOfPressure
// AI-assisted implementation: Codex (OpenAI).
// Host-free production selection and measured text layout. No input behavior.

pub(super) const MAX_ROWS: usize = 20;
const MAX_FIELD_BYTES: usize = 512;
const PAD: f32 = 2.0;
const GAP: f32 = 4.0;
const LINE: f32 = 10.0;

pub(super) fn row_limit(value: f64) -> usize {
    if value.is_finite() { value.clamp(1.0, MAX_ROWS as f64).floor() as usize } else { 8 }
}

pub(super) struct Fact<'a> {
    pub(super) name: &'a str,
    pub(super) key: Option<&'a str>,
    pub(super) on: bool,
}

#[derive(Debug, PartialEq)]
pub(super) struct Row {
    pub(super) name: String,
    pub(super) key: String,
    pub(super) on: bool,
}

#[derive(Debug, PartialEq)]
pub(super) struct Selection {
    pub(super) rows: Vec<Row>,
    pub(super) total: usize,
}

// Bound copied facts as well as the number of rows. Display invalid control
// characters as visible escapes rather than allowing a plugin key/name to
// create another HUD line. Unknown identifiers remain explicit, never guessed.
fn field(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') {
            let fragment = c.escape_default().to_string();
            if out.len() + fragment.len() > MAX_FIELD_BYTES { out.push_str("..."); break; }
            out.push_str(&fragment);
        } else {
            if out.len() + c.len_utf8() > MAX_FIELD_BYTES { out.push_str("..."); break; }
            out.push(c);
        }
    }
    out
}

pub(super) fn select<'a>(facts: impl IntoIterator<Item = Fact<'a>>, only_active: bool, limit: usize) -> Selection {
    let mut result = Selection { rows: Vec::new(), total: 0 };
    let limit = limit.clamp(1, MAX_ROWS);
    for fact in facts {
        let Some(key) = fact.key else { continue };
        if only_active && !fact.on { continue; }
        result.total = result.total.saturating_add(1);
        if result.rows.len() < limit {
            result.rows.push(Row { name: field(fact.name), key: field(key), on: fact.on });
        }
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Ink { Title, Name, Key, On, Off, Footer }

#[derive(Debug)]
pub(super) struct Text {
    pub(super) text: String,
    pub(super) at: [f32; 2],
    pub(super) ink: Ink,
}

#[derive(Debug)]
pub(super) struct Plan {
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) text: Vec<Text>,
}

fn measured(text: &str, measure: &impl Fn(&str) -> f32) -> Option<f32> {
    let width = measure(text);
    (width.is_finite() && width >= 0.0).then_some(width)
}

fn fit(text: &str, width: f32, measure: &impl Fn(&str) -> f32) -> Option<String> {
    if measured(text, measure)? <= width { return Some(text.to_owned()); }
    if measured("...", measure)? > width { return None; }
    let ends: Vec<usize> = text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len())).collect();
    let (mut lo, mut hi) = (0, ends.len());
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if measured(&format!("{}...", &text[..ends[mid]]), measure)? <= width { lo = mid; } else { hi = mid; }
    }
    Some(format!("{}...", &text[..ends[lo]]))
}

// Corners::place uses a two-pixel margin in the pinned host. A zero-height
// anchor reserves that margin; the final group reserves the next margin too.
pub(super) fn available_height(view_height: f32, anchor_y: f32, top: bool) -> Option<f32> {
    if !view_height.is_finite() || !anchor_y.is_finite() || view_height <= 0.0
        || anchor_y < PAD || anchor_y > view_height - PAD { return None; }
    let room = if top { view_height - anchor_y - 4.0 } else { anchor_y - 4.0 };
    (room >= 22.0).then_some(room)
}

pub(super) fn inside(at: [f32; 2], size: [f32; 2], view: [f32; 2]) -> bool {
    at.into_iter().chain(size).chain(view).all(f32::is_finite)
        && size[0] > 0.0 && size[1] > 0.0
        && at[0] >= PAD && at[1] >= PAD
        && at[0] + size[0] <= view[0] - PAD
        && at[1] + size[1] <= view[1] - PAD
}

pub(super) fn layout(selection: &Selection, only_active: bool, view_width: f32, room: f32, measure: impl Fn(&str) -> f32) -> Option<Plan> {
    if !view_width.is_finite() || !room.is_finite() || view_width <= 4.0 || room < 22.0 { return None; }
    let dots = measured("...", &measure)?;
    let status = measured("OFF", &measure)?.max(measured("ON", &measure)?);
    if dots <= 0.0 || status <= 0.0 { return None; }
    let max_width = (view_width - 4.0).min(240.0);
    if max_width < 2.0 * PAD + 2.0 * dots + 2.0 * GAP + status { return None; }
    let capacity = (((room - PAD) / LINE).floor() as usize).saturating_sub(1);
    let total = selection.total.max(selection.rows.len());
    let mut shown = selection.rows.len().min(capacity);
    if total > shown { shown = shown.min(capacity.saturating_sub(1)); }
    let hidden = total.saturating_sub(shown);
    let rows = &selection.rows[..shown];
    let mut name_width: f32 = 0.0;
    let mut key_width: f32 = 0.0;
    for row in rows {
        name_width = name_width.max(measured(&row.name, &measure)?);
        key_width = key_width.max(measured(&row.key, &measure)?);
    }
    let footer = if hidden > 0 {
        Some(format!("+{hidden} more"))
    } else if total == 0 {
        Some(if only_active { "No active binds" } else { "No bound modules" }.to_owned())
    } else { None };
    let footer_width = match footer.as_ref() { Some(text) => measured(text, &measure)?, None => 0.0 };
    let width = ((name_width + key_width + status + 2.0 * GAP)
        .max(measured("Keybinds", &measure)?)
        .max(footer_width)
        .max(2.0 * dots + 2.0 * GAP + status)
        + 2.0 * PAD).min(max_width);
    let inner = width - 2.0 * PAD;
    let fields = inner - status - 2.0 * GAP;
    let name_budget = name_width.min(fields / 2.0);
    let key_budget = key_width.min(fields - name_budget);
    let name_budget = fields - key_budget;
    let mut text = vec![Text { text: fit("Keybinds", inner, &measure)?, at: [PAD, PAD], ink: Ink::Title }];
    for (i, row) in rows.iter().enumerate() {
        let y = PAD + LINE * (i + 1) as f32;
        text.push(Text { text: fit(&row.name, name_budget, &measure)?, at: [PAD, y], ink: Ink::Name });
        text.push(Text { text: fit(&row.key, key_budget, &measure)?, at: [PAD + name_budget + GAP, y], ink: Ink::Key });
        let label = if row.on { "ON" } else { "OFF" };
        text.push(Text { text: label.into(), at: [width - PAD - measured(label, &measure)?, y], ink: if row.on { Ink::On } else { Ink::Off } });
    }
    if let Some(mut footer) = footer {
        if hidden > 0 && measured(&footer, &measure)? > inner {
            footer = format!("+{hidden}");
            // Never truncate an overflow count into a different number.
            if measured(&footer, &measure)? > inner { return None; }
        }
        text.push(Text { text: fit(&footer, inner, &measure)?, at: [PAD, PAD + LINE * (shown + 1) as f32], ink: Ink::Footer });
    }
    let lines = 1 + shown + usize::from(hidden > 0 || total == 0);
    let height = PAD + LINE * lines as f32;
    (height <= room).then_some(Plan { width, height, text })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn width(text: &str) -> f32 { text.chars().map(|c| if c == 'W' { 8.0 } else { 6.0 }).sum() }
    fn selection(count: usize) -> Selection {
        select((0..count).map(|i| Fact { name: if i % 2 == 0 { "Zoom" } else { "Flight" }, key: Some("Right Shift"), on: i % 2 == 0 }), false, MAX_ROWS)
    }

    #[test]
    fn selects_bound_facts_in_registration_order_without_changing_effective_state() {
        let facts = || [
            Fact { name: "Unbound", key: None, on: true },
            Fact { name: "Second", key: Some("vendor.unknown.button"), on: false },
            Fact { name: "First", key: Some("key.keyboard.z"), on: true },
        ];
        let all = select(facts(), false, 20);
        assert_eq!(all.total, 2);
        assert_eq!(all.rows[0], Row { name: "Second".into(), key: "vendor.unknown.button".into(), on: false });
        assert_eq!(all.rows[1].name, "First");
        let active = select(facts(), true, 20);
        assert_eq!(active.total, 1);
        assert_eq!(active.rows[0].name, "First");
        assert!(active.rows[0].on);
    }

    #[test]
    fn row_and_field_allocations_stay_bounded_but_overflow_count_is_exact() {
        let long = "é".repeat(10_000);
        let selected = select((0..100).map(|_| Fact { name: &long, key: Some(&long), on: true }), false, usize::MAX);
        assert_eq!(selected.total, 100);
        assert_eq!(selected.rows.len(), 20);
        for row in selected.rows {
            assert!(row.name.len() <= MAX_FIELD_BYTES + 3 && row.key.len() <= MAX_FIELD_BYTES + 3);
            assert!(row.name.ends_with("...") && row.key.ends_with("..."));
        }
        assert_eq!(row_limit(f64::NAN), 8);
        assert_eq!(row_limit(f64::INFINITY), 8);
        assert_eq!(row_limit(-1.0), 1);
        assert_eq!(row_limit(1.9), 1);
        assert_eq!(row_limit(1000.0), 20);
        let escaped = select([Fact { name: "A\nB", key: Some("vendor:\u{2028}\t"), on: false }], false, 1);
        assert_eq!(escaped.rows[0].name, "A\\nB");
        assert!(!escaped.rows[0].key.chars().any(|c| c.is_control() || c == '\u{2028}'));
    }

    #[test]
    fn measured_columns_keep_status_complete_and_long_labels_inside_the_panel() {
        let selected = select([
            Fact { name: "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWW", key: Some("Vendor Device Very Long Unknown Button"), on: true },
            Fact { name: "Second very long module name", key: Some("Another Very Long Binding Label"), on: false },
        ], false, 20);
        let plan = layout(&selected, false, 150.0, 200.0, width).unwrap();
        assert!(plan.width <= 146.0);
        assert!(plan.text.iter().any(|text| text.ink == Ink::Name && text.text.ends_with("...")));
        assert!(plan.text.iter().any(|text| text.ink == Ink::Key && text.text.ends_with("...")));
        assert!(plan.text.iter().any(|text| text.ink == Ink::On && text.text == "ON"));
        assert!(plan.text.iter().any(|text| text.ink == Ink::Off && text.text == "OFF"));
        for text in plan.text {
            assert!(text.at[0] >= PAD && text.at[0] + width(&text.text) <= plan.width - PAD + 0.001);
            assert!(text.at[1] >= PAD && text.at[1] + 8.0 <= plan.height);
        }
    }

    #[test]
    fn height_reduction_retains_exact_omitted_count_and_never_clips() {
        let selected = selection(25);
        let plan = layout(&selected, false, 400.0, 52.0, width).unwrap();
        assert_eq!(plan.height, 52.0);
        assert_eq!(plan.text.iter().filter(|text| matches!(text.ink, Ink::On | Ink::Off)).count(), 3);
        assert!(plan.text.iter().any(|text| text.ink == Ink::Footer && text.text == "+22 more"));
        let tiny = layout(&selected, false, 400.0, 22.0, width).unwrap();
        assert_eq!(tiny.text.len(), 2); // Title and truthful overflow only.
        assert_eq!(tiny.text[1].text, "+25 more");
        assert!(layout(&selected, false, 50.0, 200.0, width).is_none());
        assert!(layout(&selected, false, 400.0, 21.9, width).is_none());
    }

    #[test]
    fn configured_limit_and_empty_filter_are_truthful() {
        let selected = select((0..12).map(|_| Fact { name: "Zoom", key: Some("Z"), on: true }), false, 2);
        let plan = layout(&selected, false, 400.0, 300.0, width).unwrap();
        assert_eq!(plan.text.iter().filter(|text| text.ink == Ink::On).count(), 2);
        assert!(plan.text.iter().any(|text| text.text == "+10 more"));
        let empty = select([Fact { name: "Bound but off", key: Some("Z"), on: false }], true, 20);
        let plan = layout(&empty, true, 400.0, 300.0, width).unwrap();
        assert_eq!(plan.text[1].text, "No active binds");
        let empty = select([], false, 20);
        assert_eq!(layout(&empty, false, 400.0, 300.0, width).unwrap().text[1].text, "No bound modules");
    }

    #[test]
    fn corner_room_and_final_bounds_reject_invalid_or_exhausted_space() {
        assert_eq!(available_height(300.0, 2.0, true), Some(294.0));
        assert_eq!(available_height(300.0, 298.0, false), Some(294.0));
        assert_eq!(available_height(300.0, 272.0, true), Some(24.0));
        assert!(available_height(300.0, 280.0, true).is_none());
        assert!(available_height(300.0, -2.0, false).is_none());
        assert!(available_height(f32::NAN, 2.0, true).is_none());
        assert!(inside([2.0, 4.0], [100.0, 22.0], [200.0, 30.0]));
        assert!(!inside([2.0, -1.0], [100.0, 22.0], [200.0, 30.0]));
        assert!(!inside([2.0, 4.0], [100.0, 30.0], [200.0, 30.0]));
        assert!(layout(&selection(2), false, 400.0, 100.0, |_| f32::NAN).is_none());
    }
}
