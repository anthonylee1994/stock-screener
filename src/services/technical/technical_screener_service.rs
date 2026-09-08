//! Prices -> indicators -> technical scores.

use crate::models::TechnicalRow;
use crate::services::technical::technical_indicator_calculator::calculate_indicators;
use crate::services::technical::technical_price_client::{PriceSource, TechnicalPriceClient};
use crate::services::technical::technical_score_calculator::add_scores;
use crate::utils::ticker_normalizer::normalize_ticker_list;

/// The stage the integrated builder consumes.
#[allow(async_fn_in_trait)]
pub trait TechnicalSource {
    async fn run(&self, tickers: &[String]) -> Vec<TechnicalRow>;
}

pub struct TechnicalScreenerService<P: PriceSource> {
    price_client: P,
}

impl Default for TechnicalScreenerService<TechnicalPriceClient> {
    fn default() -> Self {
        Self::new(TechnicalPriceClient::new())
    }
}

impl<P: PriceSource> TechnicalScreenerService<P> {
    pub fn new(price_client: P) -> Self {
        TechnicalScreenerService { price_client }
    }
}

impl<P: PriceSource> TechnicalSource for TechnicalScreenerService<P> {
    async fn run(&self, tickers: &[String]) -> Vec<TechnicalRow> {
        let normalized_tickers = normalize_ticker_list(tickers);
        if normalized_tickers.is_empty() {
            return Vec::new();
        }

        let price_data = match self
            .price_client
            .download_price_data(&normalized_tickers)
            .await
        {
            Ok(price_data) => price_data,
            Err(error) => {
                tracing::error!(
                    "獲取技術面價格資料失敗 tickers={}: {}",
                    normalized_tickers.len(),
                    error
                );
                return Vec::new();
            }
        };

        let mut indicator_rows = calculate_indicators(&price_data, &normalized_tickers);
        if indicator_rows.is_empty() {
            return Vec::new();
        }
        add_scores(&mut indicator_rows);
        indicator_rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::technical::technical_price_client::{PriceData, PriceSeries};
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingPriceClient {
        calls: Mutex<Vec<Vec<String>>>,
        price_data: PriceData,
    }

    impl PriceSource for RecordingPriceClient {
        async fn download_price_data(&self, tickers: &[String]) -> anyhow::Result<PriceData> {
            self.calls.lock().unwrap().push(tickers.to_vec());
            Ok(self.price_data.clone())
        }
    }

    #[tokio::test]
    async fn returns_no_rows_for_blank_tickers() {
        let service = TechnicalScreenerService::new(RecordingPriceClient::default());

        assert!(
            service
                .run(&["".to_string(), " ".to_string()])
                .await
                .is_empty()
        );
        assert!(service.price_client.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn normalizes_tickers_before_downloading() {
        let service = TechnicalScreenerService::new(RecordingPriceClient::default());

        let result = service
            .run(&[" aapl ".to_string(), "AAPL".to_string(), "msft".to_string()])
            .await;

        assert_eq!(
            *service.price_client.calls.lock().unwrap(),
            vec![vec!["AAPL".to_string(), "MSFT".to_string()]]
        );
        assert!(result.is_empty());
    }

    struct FailingPriceClient;

    impl PriceSource for FailingPriceClient {
        async fn download_price_data(&self, _tickers: &[String]) -> anyhow::Result<PriceData> {
            Err(anyhow::anyhow!("download failed"))
        }
    }

    #[tokio::test]
    async fn returns_no_rows_on_price_error() {
        let service = TechnicalScreenerService::new(FailingPriceClient);

        assert!(service.run(&["aapl".to_string()]).await.is_empty());
    }

    #[tokio::test]
    async fn scores_indicator_rows() {
        let price_data: PriceData = [(
            "AAPL".to_string(),
            PriceSeries {
                close: (1..=202).map(|value| Some(value as f64)).collect(),
                volume: vec![Some(1_000.0); 202],
            },
        )]
        .into_iter()
        .collect();
        let service = TechnicalScreenerService::new(RecordingPriceClient {
            calls: Mutex::new(Vec::new()),
            price_data,
        });

        let result = service.run(&["AAPL".to_string()]).await;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].ticker, "AAPL");
        assert_eq!(result[0].technical_score, Some(100.0));
    }
}
