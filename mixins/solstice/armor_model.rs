// SPDX-License-Identifier: MIT
// Copyright (c) 2026 PointOfPressure
// AI-assisted implementation: Codex (OpenAI).
// Host-free policy shared by the native armor adapter and its tests.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Number {
    Missing,
    Value(i64),
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Durability {
    Empty,
    Unknown,
    Unbreakable,
    NotDamageable,
    Remaining { remaining: u32, max: u32 },
}

pub(super) fn visible(
    enabled: bool,
    native_enabled: bool,
    window: bool,
    in_world: bool,
    gui_visible: bool,
    corner_available: bool,
) -> bool {
    enabled && native_enabled && window && in_world && gui_visible && corner_available
}

pub(super) fn durability(
    empty: bool,
    known_item: bool,
    unbreakable: Option<bool>,
    max: Number,
    damage: Number,
) -> Durability {
    if empty {
        return Durability::Empty;
    }
    if !known_item || unbreakable.is_none() {
        return Durability::Unknown;
    }
    if unbreakable == Some(true) {
        return Durability::Unbreakable;
    }
    match (max, damage) {
        (Number::Invalid, _) | (_, Number::Invalid) => Durability::Unknown,
        (Number::Missing, _) | (_, Number::Missing) => Durability::NotDamageable,
        (Number::Value(max), Number::Value(damage))
            if max > 0 && max <= i64::from(i32::MAX) && damage >= 0 && damage <= max =>
        {
            Durability::Remaining { remaining: (max - damage) as u32, max: max as u32 }
        }
        _ => Durability::Unknown,
    }
}

impl Durability {
    pub(super) fn text(self, slot: &str, display: &str) -> String {
        let value = match self {
            Self::Empty => "empty".to_owned(),
            Self::Unknown => "unknown".to_owned(),
            Self::Unbreakable => "unbreakable".to_owned(),
            Self::NotDamageable => "no durability".to_owned(),
            Self::Remaining { remaining, max } => {
                // Floor the percentage: a damaged item never claims 100%.
                let percent = u64::from(remaining) * 100 / u64::from(max);
                match display {
                    "Remaining" => format!("{remaining}/{max}"),
                    "Percent" => format!("{percent}%"),
                    _ => format!("{remaining}/{max} ({percent}%)"),
                }
            }
        };
        format!("{slot}: {value}")
    }
}

#[cfg(test)]
mod tests {
    use super::{Durability::*, Number::*, *};

    #[test]
    fn visibility_requires_every_gate() {
        assert!(visible(true, true, true, true, true, true));
        for off in 0..6 {
            let mut gates = [true; 6];
            gates[off] = false;
            let [a, b, c, d, e, f] = gates;
            assert!(!visible(a, b, c, d, e, f));
        }
    }

    #[test]
    fn all_equipment_statuses_are_explicit() {
        assert_eq!(durability(true, false, None, Missing, Missing), Empty);
        assert_eq!(durability(false, false, Some(false), Value(363), Value(0)), Unknown);
        assert_eq!(durability(false, true, None, Value(363), Value(0)), Unknown);
        assert_eq!(durability(false, true, Some(true), Value(363), Value(99)), Unbreakable);
        assert_eq!(durability(false, true, Some(false), Missing, Value(0)), NotDamageable);
        assert_eq!(durability(false, true, Some(false), Value(363), Missing), NotDamageable);
        assert_eq!(Empty.text("Head", "Both"), "Head: empty");
        assert_eq!(Unknown.text("Chest", "Both"), "Chest: unknown");
        assert_eq!(Unbreakable.text("Legs", "Both"), "Legs: unbreakable");
        assert_eq!(NotDamageable.text("Feet", "Both"), "Feet: no durability");
    }

    #[test]
    fn actual_custom_max_and_damage_determine_remaining() {
        let d = durability(false, true, Some(false), Value(2000), Value(251));
        assert_eq!(d, Remaining { remaining: 1749, max: 2000 });
        assert_eq!(d.text("Chest", "Both"), "Chest: 1749/2000 (87%)");
        assert_eq!(d.text("Chest", "Remaining"), "Chest: 1749/2000");
        assert_eq!(d.text("Chest", "Percent"), "Chest: 87%");
        assert_eq!(durability(false, true, Some(false), Value(363), Value(363)), Remaining { remaining: 0, max: 363 });
        assert_eq!(durability(false, true, Some(false), Value(363), Value(0)).text("Head", "Both"), "Head: 363/363 (100%)");
        assert_eq!(durability(false, true, Some(false), Value(363), Value(1)).text("Head", "Percent"), "Head: 99%");
    }

    #[test]
    fn malformed_or_out_of_range_values_never_become_plausible_durability() {
        for (max, damage) in [
            (Invalid, Value(0)), (Value(10), Invalid), (Value(0), Value(0)),
            (Value(-1), Value(0)), (Value(10), Value(-1)), (Value(10), Value(11)),
            (Value(i64::from(i32::MAX) + 1), Value(0)), (Value(i64::MAX), Value(0)),
        ] {
            assert_eq!(durability(false, true, Some(false), max, damage), Unknown);
        }
        let d = durability(false, true, Some(false), Value(i64::from(i32::MAX)), Value(1));
        assert_eq!(d.text("Head", "Percent"), "Head: 99%");
    }
}
