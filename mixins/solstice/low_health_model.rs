// SPDX-License-Identifier: MIT
// Copyright (c) 2026 PointOfPressure
// AI-assisted implementation: Codex (OpenAI).
// Pure production policy: base HP only, with no game actions or notifications.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Config {
    pub(super) threshold: f64,
    pub(super) hysteresis: f64,
}

impl Config {
    fn valid(self) -> bool {
        self.threshold.is_finite() && (1.0..=40.0).contains(&self.threshold)
            && self.hysteresis.is_finite() && (0.0..=20.0).contains(&self.hysteresis)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Reading {
    pub(super) health: f64,
    pub(super) threshold: f64,
}

impl Reading {
    pub(super) fn text(self) -> String {
        format!("Low health: {:.1} HP (threshold {})", self.health, self.threshold)
    }
}

#[derive(Default, Debug)]
pub(super) struct Warning {
    active: bool,
    config: Option<Config>,
}

impl Warning {
    // None means disabled, no local player/world, or invalid/dead health.
    // Hidden HUD corners and F1 do not interrupt this update: they only hide
    // drawing. Lifecycle gaps therefore reset even when the HUD is hidden.
    pub(super) fn update(&mut self, config: Option<Config>, base_health: Option<f64>) -> Option<Reading> {
        let valid = config.filter(|c| c.valid()).zip(base_health.filter(|hp| {
            hp.is_finite() && *hp > 0.0 && *hp <= f64::from(f32::MAX)
        }));
        let Some((config, health)) = valid else {
            *self = Self::default();
            return None;
        };
        if self.config != Some(config) {
            self.active = false;
            self.config = Some(config);
        }
        // Strictly above the recovery boundary clears the latch. This also
        // keeps zero hysteresis stable at exactly the trigger threshold.
        self.active = health <= if self.active {
            config.threshold + config.hysteresis
        } else {
            config.threshold
        };
        self.active.then_some(Reading { health, threshold: config.threshold })
    }
}

pub(super) fn visible(active: bool, gui_visible: bool, corner_available: bool) -> bool {
    active && gui_visible && corner_available
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Option<Config> { Some(Config { threshold: 6.0, hysteresis: 2.0 }) }

    #[test]
    fn enters_at_threshold_and_recovers_above_hysteresis_boundary() {
        let mut warning = Warning::default();
        assert_eq!(warning.update(config(), Some(7.0)), None);
        assert!(warning.update(config(), Some(6.0)).is_some());
        assert!(warning.update(config(), Some(7.0)).is_some());
        assert!(warning.update(config(), Some(8.0)).is_some());
        assert_eq!(warning.update(config(), Some(8.01)), None);
        assert_eq!(warning.update(config(), Some(7.0)), None);
        assert!(warning.update(config(), Some(5.5)).is_some());
    }

    #[test]
    fn steady_warning_and_zero_hysteresis_have_no_boundary_oscillation() {
        let config = Some(Config { threshold: 6.0, hysteresis: 0.0 });
        let mut warning = Warning::default();
        for _ in 0..100 {
            assert_eq!(warning.update(config, Some(6.0)).unwrap().text(), "Low health: 6.0 HP (threshold 6)");
        }
        assert_eq!(warning.update(config, Some(6.01)), None);
        assert!(warning.update(config, Some(6.0)).is_some());
    }

    #[test]
    fn lifecycle_and_disable_gaps_reset_the_latch() {
        for gap in [(None, Some(5.0)), (config(), None), (config(), Some(0.0)), (config(), Some(-1.0))] {
            let mut warning = Warning::default();
            assert!(warning.update(config(), Some(5.0)).is_some());
            assert_eq!(warning.update(gap.0, gap.1), None);
            // 7 HP would remain active if the previous world's latch survived.
            assert_eq!(warning.update(config(), Some(7.0)), None);
        }
    }

    #[test]
    fn setting_changes_recompute_instead_of_inheriting_old_latch() {
        let mut warning = Warning::default();
        assert!(warning.update(config(), Some(5.0)).is_some());
        assert_eq!(warning.update(Some(Config { threshold: 4.0, hysteresis: 10.0 }), Some(7.0)), None);
        assert!(warning.update(Some(Config { threshold: 8.0, hysteresis: 1.0 }), Some(7.0)).is_some());
    }

    #[test]
    fn nonfinite_and_out_of_bounds_inputs_reset_safely() {
        for health in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX, 0.0, -1.0] {
            let mut warning = Warning::default();
            warning.update(config(), Some(5.0));
            assert_eq!(warning.update(config(), Some(health)), None);
            assert_eq!(warning.update(config(), Some(7.0)), None);
        }
        for invalid in [
            Config { threshold: f64::NAN, hysteresis: 2.0 },
            Config { threshold: 6.0, hysteresis: f64::INFINITY },
            Config { threshold: 0.0, hysteresis: 2.0 },
            Config { threshold: 40.1, hysteresis: 2.0 },
            Config { threshold: 6.0, hysteresis: -0.1 },
            Config { threshold: 6.0, hysteresis: 20.1 },
        ] {
            let mut warning = Warning::default();
            warning.update(config(), Some(5.0));
            assert_eq!(warning.update(Some(invalid), Some(5.0)), None);
            assert_eq!(warning.update(config(), Some(7.0)), None);
        }
        let mut warning = Warning::default();
        assert_eq!(warning.update(config(), Some(f64::from(f32::MAX))), None);
    }

    #[test]
    fn hiding_the_hud_does_not_discard_health_transitions() {
        let mut warning = Warning::default();
        let active = warning.update(config(), Some(5.0)).is_some();
        assert!(!visible(active, false, true));
        assert!(!visible(active, true, false));
        assert!(visible(active, true, true));
        assert!(!visible(false, true, true));
        // Recovery is sampled while F1 or a reserved corner hides the text.
        warning.update(config(), Some(9.0));
        assert!(!visible(warning.update(config(), Some(7.0)).is_some(), true, true));
    }
}
