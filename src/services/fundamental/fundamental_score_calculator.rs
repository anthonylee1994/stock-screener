//! Weighted percentile scoring for the fundamental metrics.

use std::collections::HashMap;

use crate::models::StockRow;
use crate::services::common::percentile_scorer::percentile_score;
use crate::services::common::score_curver::curve_score;
use crate::services::common::series_normalizer::{clip_upper, round_option, round_to};

/// One scored metric: where to read it, where to write its score, whether a
/// higher raw value is better, and its share of the fundamental score.
///
/// Weights follow the "AI 半導體 quality momentum" profile:
/// quality 45% / growth 25% / valuation 20% / leverage 10%. Market Cap only
/// produces a rank score (weight 0) so it never lifts the total on its own.
pub struct ScoreMetric {
    pub name: &'static str,
    pub metric: fn(&StockRow) -> Option<f64>,
    pub set_score: fn(&mut StockRow, Option<f64>),
    pub higher_is_better: bool,
    pub weight: f64,
}

pub const SCORE_METRICS: &[ScoreMetric] = &[
    ScoreMetric {
        name: "Market Cap",
        metric: |row| row.market_cap,
        set_score: |row, score| row.market_cap_score = score,
        higher_is_better: true,
        weight: 0.0,
    },
    ScoreMetric {
        name: "ROE",
        metric: |row| row.roe,
        set_score: |row, score| row.roe_score = score,
        higher_is_better: true,
        weight: 0.15,
    },
    ScoreMetric {
        name: "ROIC",
        metric: |row| row.roic,
        set_score: |row, score| row.roic_score = score,
        higher_is_better: true,
        weight: 0.15,
    },
    ScoreMetric {
        name: "Profit Margin",
        metric: |row| row.profit_margin,
        set_score: |row, score| row.profit_margin_score = score,
        higher_is_better: true,
        weight: 0.15,
    },
    ScoreMetric {
        name: "Gross Margin",
        metric: |row| row.gross_margin,
        set_score: |row, score| row.gross_margin_score = score,
        higher_is_better: true,
        weight: 0.0,
    },
    ScoreMetric {
        name: "EPS Past 5Y",
        metric: |row| row.eps_past_5y,
        set_score: |row, score| row.eps_past_5y_score = score,
        higher_is_better: true,
        weight: 0.15,
    },
    ScoreMetric {
        name: "Sales Past 5Y",
        metric: |row| row.sales_past_5y,
        set_score: |row, score| row.sales_past_5y_score = score,
        higher_is_better: true,
        weight: 0.10,
    },
    ScoreMetric {
        name: "Forward P/E",
        metric: |row| row.forward_pe,
        set_score: |row, score| row.forward_pe_score = score,
        higher_is_better: false,
        weight: 0.07,
    },
    ScoreMetric {
        name: "PEG",
        metric: |row| row.peg,
        set_score: |row, score| row.peg_score = score,
        higher_is_better: false,
        weight: 0.07,
    },
    ScoreMetric {
        name: "P/S",
        metric: |row| row.ps,
        set_score: |row, score| row.ps_score = score,
        higher_is_better: false,
        weight: 0.0,
    },
    ScoreMetric {
        name: "P/FCF",
        metric: |row| row.pfcf,
        set_score: |row, score| row.pfcf_score = score,
        higher_is_better: false,
        weight: 0.06,
    },
    ScoreMetric {
        name: "Debt/Equity",
        metric: |row| row.debt_equity,
        set_score: |row, score| row.debt_equity_score = score,
        higher_is_better: false,
        weight: 0.10,
    },
];

const MIN_SECTOR_SCORE_SAMPLE_SIZE: usize = 5;
const MIN_CORE_SCORE_METRIC_COUNT: usize = 3;
const INSUFFICIENT_CORE_SCORE_CAP: f64 = 60.0;
const MIN_CORE_AVERAGE_SCORE: f64 = 70.0;
const WEAK_CORE_SCORE_CAP: f64 = 75.0;
const MIN_QUALITY_AVERAGE_SCORE: f64 = 55.0;
const WEAK_QUALITY_SCORE_CAP: f64 = 70.0;
const MIN_PEG_SCORE: f64 = 35.0;
const EXPENSIVE_GROWTH_SCORE_CAP: f64 = 85.0;

/// Raw metrics that make a score trustworthy.
const CORE_SCORE_METRICS: &[fn(&StockRow) -> Option<f64>] = &[
    |row| row.roe,
    |row| row.roic,
    |row| row.profit_margin,
    |row| row.eps_past_5y,
];
const CORE_SCORE_COLUMNS: &[fn(&StockRow) -> Option<f64>] = &[
    |row| row.roe_score,
    |row| row.roic_score,
    |row| row.profit_margin_score,
    |row| row.eps_past_5y_score,
];
const QUALITY_SCORE_COLUMNS: &[fn(&StockRow) -> Option<f64>] = &[
    |row| row.roe_score,
    |row| row.roic_score,
    |row| row.profit_margin_score,
];

#[derive(Default)]
pub struct FundamentalScoreCalculator;

impl FundamentalScoreCalculator {
    /// Score every metric, combine them, curve and guard the result, then sort
    /// by fundamental score descending.
    pub fn add_score(&self, rows: &mut [StockRow]) {
        if rows.is_empty() {
            return;
        }

        // The Python version skipped metrics whose column was absent. With a
        // fixed schema the equivalent is "no metric has a single value".
        let has_any_metric = SCORE_METRICS
            .iter()
            .any(|metric| rows.iter().any(|row| (metric.metric)(row).is_some()));
        if !has_any_metric {
            for row in rows.iter_mut() {
                row.fundamental_score = Some(0.0);
            }
            return;
        }

        let sectors: Vec<Option<String>> = rows
            .iter()
            .map(|row| normalize_sector(row.sector.as_deref()))
            .collect();
        let mut weighted_score = vec![0.0; rows.len()];

        for metric in SCORE_METRICS {
            let values: Vec<Option<f64>> = rows.iter().map(|row| (metric.metric)(row)).collect();
            let raw_score = self.score_column(&values, metric.higher_is_better, &sectors);
            for (index, row) in rows.iter_mut().enumerate() {
                (metric.set_score)(row, Some(round_to(raw_score[index], 2)));
                weighted_score[index] += raw_score[index] * metric.weight;
            }
        }

        let curved = curve_score(
            &weighted_score
                .iter()
                .map(|score| Some(round_to(*score, 2)))
                .collect::<Vec<_>>(),
        );
        for (index, row) in rows.iter_mut().enumerate() {
            row.fundamental_score = round_option(curved[index], 2);
        }
        for row in rows.iter_mut() {
            row.fundamental_score = apply_score_guardrails(row);
        }

        sort_by_score_descending(rows, |row| row.fundamental_score);
    }

    /// Percentile inside the row's sector when that sector has enough samples,
    /// otherwise the whole-market percentile.
    fn score_column(
        &self,
        values: &[Option<f64>],
        higher_is_better: bool,
        sectors: &[Option<String>],
    ) -> Vec<f64> {
        let global_score = percentile_score(values, higher_is_better);
        let mut sector_score: Vec<Option<f64>> = vec![None; values.len()];

        let mut indexes_by_sector: HashMap<&str, Vec<usize>> = HashMap::new();
        for (index, sector) in sectors.iter().enumerate() {
            if let Some(sector) = sector {
                indexes_by_sector.entry(sector).or_default().push(index);
            }
        }

        for indexes in indexes_by_sector.values() {
            let sector_values: Vec<Option<f64>> =
                indexes.iter().map(|index| values[*index]).collect();
            let sample_size = sector_values.iter().filter(|value| value.is_some()).count();
            if sample_size < MIN_SECTOR_SCORE_SAMPLE_SIZE {
                continue;
            }
            let scores = percentile_score(&sector_values, higher_is_better);
            for (position, index) in indexes.iter().enumerate() {
                sector_score[*index] = Some(scores[position]);
            }
        }

        sector_score
            .into_iter()
            .enumerate()
            .map(|(index, score)| score.unwrap_or(global_score[index]))
            .collect()
    }
}

fn normalize_sector(sector: Option<&str>) -> Option<String> {
    let sector = sector?.trim();
    if sector.is_empty() {
        return None;
    }
    Some(sector.to_string())
}

/// Cap the score when the row is thin on core metrics, weak on quality, or
/// priced for growth it does not have.
fn apply_score_guardrails(row: &StockRow) -> Option<f64> {
    let mut score = row.fundamental_score;

    let core_metric_count = CORE_SCORE_METRICS
        .iter()
        .filter(|metric| metric(row).is_some())
        .count();
    if core_metric_count < MIN_CORE_SCORE_METRIC_COUNT {
        score = clip_upper(score, INSUFFICIENT_CORE_SCORE_CAP);
    }
    if average_score(row, CORE_SCORE_COLUMNS) < MIN_CORE_AVERAGE_SCORE {
        score = clip_upper(score, WEAK_CORE_SCORE_CAP);
    }
    if average_score(row, QUALITY_SCORE_COLUMNS) < MIN_QUALITY_AVERAGE_SCORE {
        score = clip_upper(score, WEAK_QUALITY_SCORE_CAP);
    }
    if row.peg_score.unwrap_or(0.0) < MIN_PEG_SCORE {
        score = clip_upper(score, EXPENSIVE_GROWTH_SCORE_CAP);
    }
    score
}

/// Mean of the present scores; an all-missing row averages to `0`.
fn average_score(row: &StockRow, columns: &[fn(&StockRow) -> Option<f64>]) -> f64 {
    let scores: Vec<f64> = columns.iter().filter_map(|column| column(row)).collect();
    if scores.is_empty() {
        return 0.0;
    }
    scores.iter().sum::<f64>() / scores.len() as f64
}

/// Stable sort, highest score first, missing scores last.
pub fn sort_by_score_descending<T>(rows: &mut [T], score: fn(&T) -> Option<f64>) {
    rows.sort_by(|left, right| match (score(left), score(right)) {
        (Some(left), Some(right)) => right
            .partial_cmp(&left)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stock(ticker: &str) -> StockRow {
        StockRow::with_ticker(ticker)
    }

    #[test]
    fn weights_are_balanced_and_sum_to_one() {
        let total: f64 = SCORE_METRICS.iter().map(|metric| metric.weight).sum();

        assert!((total - 1.0).abs() < 1e-9);
        let weights: Vec<(&str, f64)> = SCORE_METRICS
            .iter()
            .map(|metric| (metric.name, metric.weight))
            .collect();
        assert_eq!(
            weights,
            vec![
                ("Market Cap", 0.0),
                ("ROE", 0.15),
                ("ROIC", 0.15),
                ("Profit Margin", 0.15),
                ("Gross Margin", 0.0),
                ("EPS Past 5Y", 0.15),
                ("Sales Past 5Y", 0.10),
                ("Forward P/E", 0.07),
                ("PEG", 0.07),
                ("P/S", 0.0),
                ("P/FCF", 0.06),
                ("Debt/Equity", 0.10),
            ]
        );
    }

    #[test]
    fn scores_and_sorts_weighted_metrics() {
        let mut rows = vec![
            StockRow {
                market_cap: Some(1000.0),
                forward_pe: Some(50.0),
                peg: Some(3.0),
                ps: Some(20.0),
                pfcf: Some(100.0),
                roe: Some(0.10),
                roic: Some(0.05),
                profit_margin: Some(0.05),
                gross_margin: Some(0.30),
                eps_past_5y: Some(0.05),
                sales_past_5y: Some(0.02),
                debt_equity: Some(3.0),
                ..stock("WEAK")
            },
            StockRow {
                market_cap: Some(3000.0),
                forward_pe: Some(15.0),
                peg: Some(1.0),
                ps: Some(5.0),
                pfcf: Some(20.0),
                roe: Some(0.30),
                roic: Some(0.20),
                profit_margin: Some(0.25),
                gross_margin: Some(0.70),
                eps_past_5y: Some(0.25),
                sales_past_5y: Some(0.20),
                debt_equity: Some(0.2),
                ..stock("STRONG")
            },
        ];

        FundamentalScoreCalculator.add_score(&mut rows);

        let tickers: Vec<&str> = rows.iter().map(StockRow::ticker_str).collect();
        assert_eq!(tickers, ["STRONG", "WEAK"]);
        assert_eq!(rows[0].fundamental_score, Some(100.0));
        assert_eq!(rows[1].fundamental_score, Some(0.0));
        assert_eq!(rows[0].roe_score, Some(100.0));
        assert_eq!(rows[0].roic_score, Some(100.0));
        assert_eq!(rows[0].gross_margin_score, Some(100.0));
        assert_eq!(rows[0].forward_pe_score, Some(100.0));
        assert_eq!(rows[0].peg_score, Some(100.0));
        assert_eq!(rows[0].ps_score, Some(100.0));
    }

    fn sector_row(ticker: &str, sector: &str, roe: f64) -> StockRow {
        StockRow {
            sector: Some(sector.to_string()),
            roe: Some(roe),
            ..stock(ticker)
        }
    }

    fn score_by_ticker(
        rows: &[StockRow],
        reader: fn(&StockRow) -> Option<f64>,
    ) -> HashMap<String, f64> {
        rows.iter()
            .map(|row| (row.ticker_str().to_string(), reader(row).unwrap()))
            .collect()
    }

    #[test]
    fn scores_metrics_relative_to_sector() {
        let mut rows = vec![
            sector_row("TECH_LOW", "Technology", 10.0),
            sector_row("TECH_HIGH", "Technology", 20.0),
            sector_row("TECH_MID", "Technology", 15.0),
            sector_row("TECH_TOP", "Technology", 25.0),
            sector_row("TECH_BOTTOM", "Technology", 5.0),
            sector_row("ENERGY_LOW", "Energy", 100.0),
            sector_row("ENERGY_HIGH", "Energy", 500.0),
            sector_row("ENERGY_MID", "Energy", 300.0),
            sector_row("ENERGY_TOP", "Energy", 700.0),
            sector_row("ENERGY_BOTTOM", "Energy", 50.0),
        ];

        FundamentalScoreCalculator.add_score(&mut rows);
        let scores = score_by_ticker(&rows, |row| row.roe_score);

        assert_eq!(scores["TECH_TOP"], 100.0);
        assert_eq!(scores["TECH_BOTTOM"], 0.0);
        assert_eq!(scores["ENERGY_TOP"], 100.0);
        assert_eq!(scores["ENERGY_BOTTOM"], 0.0);
    }

    #[test]
    fn falls_back_to_the_global_score_for_a_small_sector() {
        let mut rows = vec![
            sector_row("TECH_LOW", "Technology", 10.0),
            sector_row("TECH_HIGH", "Technology", 20.0),
            sector_row("TECH_MID", "Technology", 15.0),
            sector_row("TECH_TOP", "Technology", 25.0),
            sector_row("TECH_BOTTOM", "Technology", 5.0),
            sector_row("HEALTH_ONLY", " Healthcare ", 100.0),
            sector_row("MISSING_SECTOR", " ", 50.0),
        ];

        FundamentalScoreCalculator.add_score(&mut rows);
        let scores = score_by_ticker(&rows, |row| row.roe_score);

        assert_eq!(scores["HEALTH_ONLY"], 100.0);
        assert_eq!(scores["MISSING_SECTOR"], 83.33);
    }

    /// Five same-sector rows so the sector percentile kicks in, matching the
    /// guardrail fixtures from the Python suite.
    fn guardrail_row(
        ticker: &str,
        quality: f64,
        eps_past_5y: f64,
        sales_past_5y: f64,
        debt_equity: f64,
    ) -> StockRow {
        StockRow {
            sector: Some("Technology".to_string()),
            roe: Some(quality),
            roic: Some(quality),
            profit_margin: Some(quality),
            eps_past_5y: Some(eps_past_5y),
            sales_past_5y: Some(sales_past_5y),
            debt_equity: Some(debt_equity),
            ..stock(ticker)
        }
    }

    #[test]
    fn caps_the_score_when_core_metrics_are_missing() {
        let mut rows = vec![
            StockRow {
                sector: Some("Technology".to_string()),
                roe: Some(100.0),
                sales_past_5y: Some(100.0),
                debt_equity: Some(0.0),
                ..stock("INSUFFICIENT_CORE")
            },
            guardrail_row("HAS_CORE", 90.0, 90.0, 90.0, 0.1),
            guardrail_row("PEER_LOW", 1.0, 1.0, 1.0, 5.0),
            guardrail_row("PEER_MID", 2.0, 2.0, 2.0, 4.0),
            guardrail_row("PEER_HIGH", 3.0, 3.0, 3.0, 3.0),
        ];

        FundamentalScoreCalculator.add_score(&mut rows);
        let scores = score_by_ticker(&rows, |row| row.fundamental_score);

        assert!(scores["INSUFFICIENT_CORE"] <= 60.0);
        assert!(scores["HAS_CORE"] > 60.0);
    }

    #[test]
    fn caps_the_score_when_core_scores_are_weak() {
        let mut rows = vec![
            guardrail_row("WEAK_CORE", 10.0, 10.0, 100.0, 0.0),
            guardrail_row("STRONG_CORE", 100.0, 100.0, 90.0, 0.1),
            guardrail_row("PEER_LOW", 5.0, 5.0, 5.0, 5.0),
            guardrail_row("PEER_MID", 30.0, 30.0, 30.0, 3.0),
            guardrail_row("PEER_HIGH", 50.0, 50.0, 50.0, 1.0),
        ];

        FundamentalScoreCalculator.add_score(&mut rows);
        let scores = score_by_ticker(&rows, |row| row.fundamental_score);

        assert!(scores["WEAK_CORE"] <= 75.0);
        assert!(scores["STRONG_CORE"] > 75.0);
    }

    #[test]
    fn caps_the_score_when_quality_is_weak() {
        let mut rows = vec![
            guardrail_row("WEAK_QUALITY", 2.0, 100.0, 100.0, 0.0),
            guardrail_row("STRONG_QUALITY", 100.0, 90.0, 90.0, 0.1),
            guardrail_row("PEER_LOW", 1.0, 1.0, 1.0, 5.0),
            guardrail_row("PEER_MID", 5.0, 5.0, 5.0, 3.0),
            guardrail_row("PEER_HIGH", 50.0, 50.0, 50.0, 1.0),
        ];

        FundamentalScoreCalculator.add_score(&mut rows);
        let scores = score_by_ticker(&rows, |row| row.fundamental_score);

        assert!(scores["WEAK_QUALITY"] <= 70.0);
        assert!(scores["STRONG_QUALITY"] > 70.0);
    }

    #[test]
    fn caps_the_score_when_peg_is_stretched() {
        let mut rows = vec![
            StockRow {
                forward_pe: Some(50.0),
                peg: Some(10.0),
                pfcf: Some(100.0),
                ..guardrail_row("EXPENSIVE_GROWTH", 100.0, 100.0, 100.0, 0.0)
            },
            StockRow {
                forward_pe: Some(1.0),
                peg: Some(0.5),
                pfcf: Some(1.0),
                ..guardrail_row("REASONABLE_GROWTH", 90.0, 90.0, 90.0, 0.1)
            },
            StockRow {
                forward_pe: Some(40.0),
                peg: Some(5.0),
                pfcf: Some(80.0),
                ..guardrail_row("PEER_LOW", 1.0, 1.0, 1.0, 5.0)
            },
            StockRow {
                forward_pe: Some(30.0),
                peg: Some(4.0),
                pfcf: Some(60.0),
                ..guardrail_row("PEER_MID", 2.0, 2.0, 2.0, 4.0)
            },
            StockRow {
                forward_pe: Some(20.0),
                peg: Some(3.0),
                pfcf: Some(40.0),
                ..guardrail_row("PEER_HIGH", 3.0, 3.0, 3.0, 3.0)
            },
        ];

        FundamentalScoreCalculator.add_score(&mut rows);
        let scores = score_by_ticker(&rows, |row| row.fundamental_score);

        assert_eq!(scores["EXPENSIVE_GROWTH"], 85.0);
        assert!(scores["REASONABLE_GROWTH"] > 85.0);
    }

    #[test]
    fn curves_the_final_score() {
        let mut rows = vec![
            StockRow {
                roe: Some(10.0),
                roic: Some(10.0),
                profit_margin: Some(10.0),
                eps_past_5y: Some(10.0),
                sales_past_5y: Some(10.0),
                debt_equity: Some(50.0),
                peg: Some(3.0),
                ..stock("LOW")
            },
            StockRow {
                roe: Some(20.0),
                roic: Some(20.0),
                profit_margin: Some(20.0),
                eps_past_5y: Some(20.0),
                sales_past_5y: Some(20.0),
                debt_equity: Some(30.0),
                peg: Some(2.0),
                ..stock("MID")
            },
            StockRow {
                roe: Some(30.0),
                roic: Some(30.0),
                profit_margin: Some(30.0),
                eps_past_5y: Some(30.0),
                sales_past_5y: Some(30.0),
                debt_equity: Some(10.0),
                peg: Some(1.0),
                ..stock("HIGH")
            },
        ];

        FundamentalScoreCalculator.add_score(&mut rows);

        let scores: Vec<Option<f64>> = rows.iter().map(|row| row.fundamental_score).collect();
        assert_eq!(scores, vec![Some(100.0), Some(50.0), Some(0.0)]);
    }

    #[test]
    fn handles_rows_without_any_scoreable_metric() {
        let mut rows = vec![stock("AAPL")];

        FundamentalScoreCalculator.add_score(&mut rows);

        assert_eq!(rows[0].fundamental_score, Some(0.0));
    }

    #[test]
    fn leaves_empty_input_alone() {
        let mut rows: Vec<StockRow> = Vec::new();

        FundamentalScoreCalculator.add_score(&mut rows);

        assert!(rows.is_empty());
    }
}
