//! Statistics helpers.

// Even-length slices use the mean of the two middle values.
pub(crate) fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    Some(if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    })
}

// Gini coefficient of non-negative values (0 = equal, towards 1 = one value dominates).
pub(crate) fn gini(values: &[f64]) -> f64 {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| *v >= 0.0).collect();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let total: f64 = sorted.iter().sum();
    if n < 2 || total == 0.0 {
        return 0.0;
    }
    let n_f = n as f64;
    let weighted: f64 = sorted
        .iter()
        .enumerate()
        .map(|(i, x)| (2.0 * (i as f64 + 1.0) - n_f - 1.0) * x)
        .sum();
    weighted / (n_f * total)
}

// Rounds halves to the nearest even integer, matching the prototype's rounding.
pub(crate) fn round_half_even(x: f64) -> f64 {
    let rounded = x.round();
    if (x - x.trunc()).abs() == 0.5 && rounded % 2.0 != 0.0 {
        rounded - x.signum()
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_odd_even_empty() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), Some(2.5));
    }

    #[test]
    fn gini_equal_and_unequal() {
        assert_eq!(gini(&[5.0, 5.0, 5.0, 5.0]), 0.0);
        assert_eq!(gini(&[1.0]), 0.0);
        assert_eq!(gini(&[0.0, 0.0]), 0.0);
        // One member does everything among four: (n - 1) / n
        assert!((gini(&[0.0, 0.0, 0.0, 10.0]) - 0.75).abs() < 1e-12);
        assert!((gini(&[1.0, 2.0, 3.0, 4.0]) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn rounding_sends_halves_to_even() {
        assert_eq!(round_half_even(0.5), 0.0);
        assert_eq!(round_half_even(1.5), 2.0);
        assert_eq!(round_half_even(2.5), 2.0);
        assert_eq!(round_half_even(72.5), 72.0);
        assert_eq!(round_half_even(73.5), 74.0);
        assert_eq!(round_half_even(72.4), 72.0);
        assert_eq!(round_half_even(72.6), 73.0);
    }
}
