//! Fetch -> clean -> normalize -> score, the whole fundamental pipeline.

use crate::models::StockRow;
use crate::services::fundamental::finviz_custom_screener::FinvizRow;
use crate::services::fundamental::fundamental_data_normalizer::FundamentalDataNormalizer;
use crate::services::fundamental::fundamental_score_calculator::FundamentalScoreCalculator;
use crate::services::fundamental::fundamental_screener_client::FundamentalScreenerClient;

/// Anything that can hand back raw Finviz screener rows.
#[allow(async_fn_in_trait)]
pub trait FundamentalRowSource {
    async fn fetch(&self, limit: i64) -> anyhow::Result<Vec<FinvizRow>>;
}

impl FundamentalRowSource for FundamentalScreenerClient {
    async fn fetch(&self, limit: i64) -> anyhow::Result<Vec<FinvizRow>> {
        FundamentalScreenerClient::fetch(self, limit).await
    }
}

/// The stage the integrated builder consumes.
#[allow(async_fn_in_trait)]
pub trait FundamentalSource {
    async fn run(&self, limit: i64) -> Vec<StockRow>;
}

pub struct FundamentalScreenerService<C: FundamentalRowSource> {
    client: C,
    normalizer: FundamentalDataNormalizer,
    score_calculator: FundamentalScoreCalculator,
}

impl Default for FundamentalScreenerService<FundamentalScreenerClient> {
    fn default() -> Self {
        Self::new(FundamentalScreenerClient::new())
    }
}

impl<C: FundamentalRowSource> FundamentalScreenerService<C> {
    pub fn new(client: C) -> Self {
        FundamentalScreenerService {
            client,
            normalizer: FundamentalDataNormalizer,
            score_calculator: FundamentalScoreCalculator,
        }
    }
}

impl<C: FundamentalRowSource> FundamentalSource for FundamentalScreenerService<C> {
    async fn run(&self, limit: i64) -> Vec<StockRow> {
        let fetched = match self.client.fetch(limit).await {
            Ok(rows) => rows,
            Err(error) => {
                tracing::error!("獲取基本面篩選器資料失敗: {}", error);
                return Vec::new();
            }
        };
        let cleaned = self.normalizer.remove_invalid_rows(fetched);
        let mut normalized = self.normalizer.normalize(cleaned);
        self.score_calculator.add_score(&mut normalized);
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::fundamental::finviz_custom_screener::FinvizCell;

    struct FakeClient {
        rows: Vec<FinvizRow>,
    }

    impl FundamentalRowSource for FakeClient {
        async fn fetch(&self, _limit: i64) -> anyhow::Result<Vec<FinvizRow>> {
            Ok(self.rows.clone())
        }
    }

    fn row(ticker: &str, peg: &str) -> FinvizRow {
        [
            ("Ticker".to_string(), FinvizCell::Text(ticker.to_string())),
            ("PEG".to_string(), FinvizCell::Text(peg.to_string())),
        ]
        .into_iter()
        .collect()
    }

    #[tokio::test]
    async fn runs_fetch_clean_normalize_and_score() {
        let service = FundamentalScreenerService::new(FakeClient {
            rows: vec![row("AAPL", "2"), row("", "3")],
        });

        let result = service.run(25).await;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].ticker_str(), "AAPL");
        assert_eq!(result[0].peg, Some(2.0));
        // Only PEG is scoreable, so the core-metric guardrail caps the row.
        assert_eq!(result[0].fundamental_score, Some(60.0));
    }

    struct FailingClient;

    impl FundamentalRowSource for FailingClient {
        async fn fetch(&self, _limit: i64) -> anyhow::Result<Vec<FinvizRow>> {
            Err(anyhow::anyhow!("network failed"))
        }
    }

    #[tokio::test]
    async fn returns_no_rows_when_the_client_fails() {
        let service = FundamentalScreenerService::new(FailingClient);

        assert!(service.run(10).await.is_empty());
    }
}
