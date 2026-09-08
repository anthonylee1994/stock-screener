//! Long-, mid- and short-term momentum scores and their weighted total.

use crate::models::TechnicalRow;
use crate::services::common::percentile_scorer::percentile_score;
use crate::services::common::score_curver::curve_score;
use crate::services::common::series_normalizer::{round_option, round_to};
use crate::services::fundamental::fundamental_score_calculator::sort_by_score_descending;

/// Each term score averages two percentile-ranked indicators.
struct TermScore {
    first: fn(&TechnicalRow) -> Option<f64>,
    second: fn(&TechnicalRow) -> Option<f64>,
    set_score: fn(&mut TechnicalRow, Option<f64>),
    weight: f64,
}

const TERM_SCORES: &[TermScore] = &[
    TermScore {
        first: |row| row.ema200_distance,
        second: |row| row.roc125,
        set_score: |row, score| row.long_term_score = score,
        weight: 0.6,
    },
    TermScore {
        first: |row| row.ema50_distance,
        second: |row| row.roc20,
        set_score: |row, score| row.mid_term_score = score,
        weight: 0.3,
    },
    TermScore {
        first: |row| row.ppo_slope3,
        second: |row| row.rsi14,
        set_score: |row, score| row.short_term_score = score,
        weight: 0.1,
    },
];

pub fn add_scores(rows: &mut [TechnicalRow]) {
    if rows.is_empty() {
        return;
    }

    let mut weighted_score = vec![0.0; rows.len()];
    for term in TERM_SCORES {
        let first: Vec<Option<f64>> = rows.iter().map(|row| (term.first)(row)).collect();
        let second: Vec<Option<f64>> = rows.iter().map(|row| (term.second)(row)).collect();
        let first_score = percentile_score(&first, true);
        let second_score = percentile_score(&second, true);

        for (index, row) in rows.iter_mut().enumerate() {
            let term_score = (first_score[index] + second_score[index]) / 2.0;
            (term.set_score)(row, Some(round_to(term_score, 2)));
            weighted_score[index] += term_score * term.weight;
        }
    }

    let curved = curve_score(&weighted_score.iter().copied().map(Some).collect::<Vec<_>>());
    for (index, row) in rows.iter_mut().enumerate() {
        row.technical_score = round_option(curved[index], 2);
    }

    sort_by_score_descending(rows, |row| row.technical_score);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(ticker: &str, indicator: f64) -> TechnicalRow {
        TechnicalRow {
            ticker: ticker.to_string(),
            quote_price: Some(indicator * 10.0),
            ema200_distance: Some(indicator),
            roc125: Some(indicator),
            ema50_distance: Some(indicator),
            roc20: Some(indicator),
            ppo_slope3: Some(indicator),
            rsi14: Some(indicator),
            ..TechnicalRow::default()
        }
    }

    #[test]
    fn scores_percentiles_and_sorts() {
        let mut rows = vec![row("LOW", 1.0), row("HIGH", 2.0)];

        add_scores(&mut rows);

        let tickers: Vec<&str> = rows.iter().map(|row| row.ticker.as_str()).collect();
        assert_eq!(tickers, ["HIGH", "LOW"]);
        assert_eq!(rows[0].long_term_score, Some(100.0));
        assert_eq!(rows[0].mid_term_score, Some(100.0));
        assert_eq!(rows[0].short_term_score, Some(100.0));
        assert_eq!(rows[0].technical_score, Some(100.0));
    }

    #[test]
    fn curves_the_final_score() {
        let mut rows = vec![row("LOW", 1.0), row("MID", 2.0), row("HIGH", 3.0)];

        add_scores(&mut rows);

        let tickers: Vec<&str> = rows.iter().map(|row| row.ticker.as_str()).collect();
        assert_eq!(tickers, ["HIGH", "MID", "LOW"]);
        let scores: Vec<Option<f64>> = rows.iter().map(|row| row.technical_score).collect();
        assert_eq!(scores, vec![Some(100.0), Some(50.0), Some(0.0)]);
    }

    #[test]
    fn leaves_empty_input_alone() {
        let mut rows: Vec<TechnicalRow> = Vec::new();

        add_scores(&mut rows);

        assert!(rows.is_empty());
    }
}
