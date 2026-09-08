//! Min-max curve that stretches a raw score series onto `0`-`100`.

/// Missing values stay missing. A single valid value, or a flat series, curves
/// to `100`.
pub fn curve_score(score: &[Option<f64>]) -> Vec<Option<f64>> {
    let valid: Vec<f64> = score.iter().flatten().copied().collect();
    if valid.is_empty() {
        return vec![None; score.len()];
    }

    let min_score = valid.iter().copied().fold(f64::INFINITY, f64::min);
    let max_score = valid.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if valid.len() == 1 || min_score == max_score {
        return score.iter().map(|value| value.map(|_| 100.0)).collect();
    }

    score
        .iter()
        .map(|value| value.map(|value| ((value - min_score) / (max_score - min_score)) * 100.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_valid_scores_to_zero_and_one_hundred() {
        let curved = curve_score(&[Some(10.0), Some(20.0), Some(30.0), None]);
        assert_eq!(curved, vec![Some(0.0), Some(50.0), Some(100.0), None]);
    }

    #[test]
    fn returns_one_hundred_for_single_or_flat_valid_score() {
        assert_eq!(curve_score(&[None, Some(42.0)]), vec![None, Some(100.0)]);
        assert_eq!(
            curve_score(&[Some(7.0), Some(7.0)]),
            vec![Some(100.0), Some(100.0)]
        );
    }

    #[test]
    fn returns_all_missing_for_an_empty_sample() {
        assert_eq!(curve_score(&[None, None]), vec![None, None]);
    }
}
