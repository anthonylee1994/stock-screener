//! Percentile scoring, matching `pandas.Series.rank(method="average")`.

/// Score every value from `0` to `100` by its average rank inside the sample.
///
/// Missing values score `0`, a single valid value scores `100`.
pub fn percentile_score(data: &[Option<f64>], ascending: bool) -> Vec<f64> {
    let valid_indexes: Vec<usize> = data
        .iter()
        .enumerate()
        .filter(|(_, value)| value.is_some())
        .map(|(index, _)| index)
        .collect();
    let valid_count = valid_indexes.len();

    if valid_count == 0 {
        return vec![0.0; data.len()];
    }
    if valid_count == 1 {
        return data
            .iter()
            .map(|value| if value.is_some() { 100.0 } else { 0.0 })
            .collect();
    }

    let ranks = average_ranks(data, &valid_indexes, ascending);
    let mut scores = vec![0.0; data.len()];
    for (index, rank) in ranks {
        scores[index] = ((rank - 1.0) / (valid_count as f64 - 1.0)) * 100.0;
    }
    scores
}

/// `(index, rank)` pairs for the valid values only. Ties share the average of
/// the ranks they span.
fn average_ranks(
    data: &[Option<f64>],
    valid_indexes: &[usize],
    ascending: bool,
) -> Vec<(usize, f64)> {
    let mut ordered: Vec<usize> = valid_indexes.to_vec();
    ordered.sort_by(|left, right| {
        let left_value = data[*left].unwrap();
        let right_value = data[*right].unwrap();
        let ordering = left_value
            .partial_cmp(&right_value)
            .unwrap_or(std::cmp::Ordering::Equal);
        if ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });

    let mut ranks = Vec::with_capacity(ordered.len());
    let mut position = 0;
    while position < ordered.len() {
        let value = data[ordered[position]].unwrap();
        let mut tie_end = position + 1;
        while tie_end < ordered.len() && data[ordered[tie_end]].unwrap() == value {
            tie_end += 1;
        }
        // Ranks are 1-based, so the tie spans positions `position + 1 ..= tie_end`.
        let average_rank = ((position + 1 + tie_end) as f64) / 2.0;
        for index in &ordered[position..tie_end] {
            ranks.push((*index, average_rank));
        }
        position = tie_end;
    }
    ranks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_missing_and_single_valid_values() {
        assert_eq!(percentile_score(&[None, None], true), vec![0.0, 0.0]);
        assert_eq!(
            percentile_score(&[None, Some(42.0)], true),
            vec![0.0, 100.0]
        );
    }

    #[test]
    fn spreads_scores_evenly_across_the_sample() {
        let scores = percentile_score(&[Some(1.0), Some(2.0), Some(3.0)], true);
        assert_eq!(scores, vec![0.0, 50.0, 100.0]);
    }

    #[test]
    fn reverses_scores_when_lower_is_better() {
        let scores = percentile_score(&[Some(1.0), Some(2.0), Some(3.0)], false);
        assert_eq!(scores, vec![100.0, 50.0, 0.0]);
    }

    #[test]
    fn averages_tied_ranks() {
        let scores = percentile_score(&[Some(1.0), Some(1.0), Some(3.0)], true);
        assert_eq!(scores, vec![25.0, 25.0, 100.0]);
    }
}
