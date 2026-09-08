//! Combine the fundamental and technical stages into the persisted universe.

use std::collections::HashMap;
use std::time::Instant;

use crate::models::StockRow;
use crate::services::common::score_curver::curve_score;
use crate::services::common::series_normalizer::round_option;
use crate::services::fundamental::fundamental_screener_service::FundamentalSource;
use crate::services::integrated::potential_stock_filter;
use crate::services::technical::technical_screener_service::TechnicalSource;

const FUNDAMENTAL_SCORE_WEIGHT: f64 = 0.75;
const TECHNICAL_SCORE_WEIGHT: f64 = 0.25;

pub struct IntegratedScreenerBuilder<F: FundamentalSource, T: TechnicalSource> {
    fundamental_service: F,
    technical_service: T,
}

impl<F: FundamentalSource, T: TechnicalSource> IntegratedScreenerBuilder<F, T> {
    pub fn new(fundamental_service: F, technical_service: T) -> Self {
        IntegratedScreenerBuilder {
            fundamental_service,
            technical_service,
        }
    }

    pub async fn build(&self, limit: i64) -> Vec<StockRow> {
        let started_at = Instant::now();
        tracing::info!("開始建立完整篩選器資料 limit={}", limit);

        let mut rows = self.get_fundamental_data(limit).await;
        if rows.is_empty() || !rows.iter().any(|row| row.ticker.is_some()) {
            tracing::warn!(
                "完整篩選器資料冇 ticker rows={} elapsed={:.2}s",
                rows.len(),
                started_at.elapsed().as_secs_f64()
            );
            return rows;
        }

        let tickers: Vec<String> = rows.iter().filter_map(|row| row.ticker.clone()).collect();
        let technical_rows = self.get_technical_data(&tickers).await;
        if !technical_rows.is_empty() {
            let technical_by_ticker: HashMap<&str, _> = technical_rows
                .iter()
                .map(|row| (row.ticker.as_str(), row))
                .collect();
            for row in rows.iter_mut() {
                if let Some(technical) = technical_by_ticker.get(row.ticker_str()) {
                    row.merge_technical(technical);
                }
            }
        }

        add_total_score(&mut rows);
        let potential_flags = potential_stock_filter::apply(&rows);
        for (row, potential) in rows.iter_mut().zip(potential_flags) {
            row.potential_stock = Some(potential);
        }

        tracing::info!(
            "完整篩選器資料已建立 limit={} rows={} elapsed={:.2}s",
            limit,
            rows.len(),
            started_at.elapsed().as_secs_f64()
        );
        rows
    }

    async fn get_fundamental_data(&self, limit: i64) -> Vec<StockRow> {
        tracing::info!("獲取基本面篩選器資料 limit={}", limit);
        let started_at = Instant::now();
        let rows = self.fundamental_service.run(limit).await;
        tracing::info!(
            "已獲取基本面篩選器資料 rows={} elapsed={:.2}s",
            rows.len(),
            started_at.elapsed().as_secs_f64()
        );
        rows
    }

    async fn get_technical_data(&self, tickers: &[String]) -> Vec<crate::models::TechnicalRow> {
        tracing::info!("獲取技術面篩選器資料 tickers={}", tickers.len());
        let started_at = Instant::now();
        let rows = self.technical_service.run(tickers).await;
        tracing::info!(
            "已獲取技術面篩選器資料 rows={} elapsed={:.2}s",
            rows.len(),
            started_at.elapsed().as_secs_f64()
        );
        rows
    }
}

/// `Total Score` weights the fundamental score at 75% and the technical score
/// at 25%, then curves the result onto `0`-`100`. A row missing either input
/// gets no total score at all.
pub fn add_total_score(rows: &mut [StockRow]) {
    if rows.is_empty() {
        return;
    }

    let raw_score: Vec<Option<f64>> = rows
        .iter()
        .map(|row| match (row.fundamental_score, row.technical_score) {
            (Some(fundamental), Some(technical)) => {
                Some(fundamental * FUNDAMENTAL_SCORE_WEIGHT + technical * TECHNICAL_SCORE_WEIGHT)
            }
            _ => None,
        })
        .collect();

    let curved = curve_score(&raw_score);
    for (row, score) in rows.iter_mut().zip(curved) {
        row.total_score = round_option(score, 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TechnicalRow;
    use std::sync::Mutex;

    struct FakeFundamentalService {
        rows: Vec<StockRow>,
        calls: Mutex<Vec<i64>>,
    }

    impl FundamentalSource for FakeFundamentalService {
        async fn run(&self, limit: i64) -> Vec<StockRow> {
            self.calls.lock().unwrap().push(limit);
            self.rows.clone()
        }
    }

    struct FakeTechnicalService {
        rows: Vec<TechnicalRow>,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl TechnicalSource for FakeTechnicalService {
        async fn run(&self, tickers: &[String]) -> Vec<TechnicalRow> {
            self.calls.lock().unwrap().push(tickers.to_vec());
            self.rows.clone()
        }
    }

    fn fundamental(ticker: &str, score: f64) -> StockRow {
        StockRow {
            fundamental_score: Some(score),
            ..StockRow::with_ticker(ticker)
        }
    }

    fn technical(ticker: &str, score: f64) -> TechnicalRow {
        TechnicalRow {
            ticker: ticker.to_string(),
            technical_score: Some(score),
            ..TechnicalRow::default()
        }
    }

    fn builder(
        fundamental_rows: Vec<StockRow>,
        technical_rows: Vec<TechnicalRow>,
    ) -> IntegratedScreenerBuilder<FakeFundamentalService, FakeTechnicalService> {
        IntegratedScreenerBuilder::new(
            FakeFundamentalService {
                rows: fundamental_rows,
                calls: Mutex::new(Vec::new()),
            },
            FakeTechnicalService {
                rows: technical_rows,
                calls: Mutex::new(Vec::new()),
            },
        )
    }

    #[tokio::test]
    async fn merges_technical_data_and_adds_a_weighted_score() {
        let builder = builder(
            vec![fundamental("AAPL", 90.0), fundamental("MSFT", 70.0)],
            vec![technical("AAPL", 50.0), technical("MSFT", 100.0)],
        );

        let result = builder.build(2).await;

        assert_eq!(*builder.fundamental_service.calls.lock().unwrap(), vec![2]);
        assert_eq!(
            *builder.technical_service.calls.lock().unwrap(),
            vec![vec!["AAPL".to_string(), "MSFT".to_string()]]
        );
        let tickers: Vec<&str> = result.iter().map(StockRow::ticker_str).collect();
        assert_eq!(tickers, ["AAPL", "MSFT"]);
        let scores: Vec<Option<f64>> = result.iter().map(|row| row.total_score).collect();
        assert_eq!(scores, vec![Some(100.0), Some(0.0)]);
    }

    #[tokio::test]
    async fn returns_fundamental_rows_untouched_without_a_ticker() {
        let builder = builder(
            vec![StockRow {
                fundamental_score: Some(90.0),
                ..StockRow::default()
            }],
            Vec::new(),
        );

        let result = builder.build(2).await;

        assert!(builder.technical_service.calls.lock().unwrap().is_empty());
        assert_eq!(result[0].total_score, None);
        assert_eq!(result[0].potential_stock, None);
    }

    #[tokio::test]
    async fn leaves_the_total_score_empty_when_technical_data_is_missing() {
        let builder = builder(vec![fundamental("AAPL", 90.0)], Vec::new());

        let result = builder.build(1).await;

        assert_eq!(
            *builder.technical_service.calls.lock().unwrap(),
            vec![vec!["AAPL".to_string()]]
        );
        assert_eq!(result[0].total_score, None);
    }

    #[test]
    fn curves_the_total_score_between_zero_and_one_hundred() {
        let mut rows = vec![
            StockRow {
                technical_score: Some(50.0),
                ..fundamental("LOW", 50.0)
            },
            StockRow {
                technical_score: Some(70.0),
                ..fundamental("MID", 70.0)
            },
            StockRow {
                technical_score: Some(90.0),
                ..fundamental("HIGH", 90.0)
            },
        ];

        add_total_score(&mut rows);

        let scores: Vec<Option<f64>> = rows.iter().map(|row| row.total_score).collect();
        assert_eq!(scores, vec![Some(0.0), Some(50.0), Some(100.0)]);
    }

    #[test]
    fn curves_a_single_valid_total_score_to_one_hundred() {
        let mut rows = vec![
            StockRow {
                technical_score: Some(50.0),
                ..fundamental("VALID", 50.0)
            },
            fundamental("MISSING", 70.0),
        ];

        add_total_score(&mut rows);

        assert_eq!(rows[0].total_score, Some(100.0));
        assert_eq!(rows[1].total_score, None);
    }

    #[test]
    fn leaves_empty_input_alone() {
        let mut rows: Vec<StockRow> = Vec::new();

        add_total_score(&mut rows);

        assert!(rows.is_empty());
    }
}
