//! Corrects stage-1 estimates from mean pace to median pace.

use uma_stable_match_scoring::{score_club, Club, ScoringOptions};

use crate::pool::ListedClub;

/// Samples needed before the factor is used.
pub const MIN_CALIBRATION_SAMPLES: usize = 20;

/// Factor that turns a club's mean pace estimate into a median estimate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibration {
    factor: f64,
    samples: usize,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            factor: 1.0,
            samples: 0,
        }
    }
}

impl Calibration {
    /// Median of the ratios, or 1.0 with fewer than [`MIN_CALIBRATION_SAMPLES`]. Zero, negative
    /// and non-finite ratios are ignored.
    pub fn from_ratios(ratios: impl IntoIterator<Item = f64>) -> Self {
        let mut ratios: Vec<f64> = ratios
            .into_iter()
            .filter(|r| r.is_finite() && *r > 0.0)
            .collect();
        let samples = ratios.len();
        if samples < MIN_CALIBRATION_SAMPLES {
            return Self {
                factor: 1.0,
                samples,
            };
        }
        ratios.sort_by(f64::total_cmp);
        let mid = samples / 2;
        let factor = if samples.is_multiple_of(2) {
            (ratios[mid - 1] + ratios[mid]) / 2.0
        } else {
            ratios[mid]
        };
        Self { factor, samples }
    }

    /// A club's true median ÷ its raw rate, or `None` if it can't be scored.
    pub fn ratio(listed: &ListedClub, club: &Club) -> Option<f64> {
        let options = ScoringOptions::default().with_include_closed(true);
        let scored = score_club(1.0, club, &options).ok()??;
        let ratio = scored.median_per_day / listed.raw_rate;
        (ratio.is_finite() && ratio > 0.0).then_some(ratio)
    }

    /// The correction factor.
    pub fn factor(&self) -> f64 {
        self.factor
    }

    /// Valid samples used.
    pub fn samples(&self) -> usize {
        self.samples
    }

    /// Whether there were enough samples.
    pub fn is_calibrated(&self) -> bool {
        self.samples >= MIN_CALIBRATION_SAMPLES
    }
}

#[cfg(test)]
mod tests;
