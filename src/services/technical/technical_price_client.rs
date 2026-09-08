//! Daily price history from Yahoo Finance, replacing `yfinance`.

use std::collections::HashMap;
use std::time::Duration;

use futures::StreamExt;
use serde::Deserialize;

const CHART_URL: &str = "https://query2.finance.yahoo.com/v8/finance/chart/";
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
pub const DEFAULT_RANGE: &str = "1y";
pub const DEFAULT_INTERVAL: &str = "1d";
pub const DOWNLOAD_RETRY_ATTEMPTS: usize = 2;
/// How many Yahoo requests are in flight at once.
const DOWNLOAD_CONCURRENCY: usize = 16;

/// Adjusted closes and volumes for one ticker, oldest bar first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PriceSeries {
    pub close: Vec<Option<f64>>,
    pub volume: Vec<Option<f64>>,
}

impl PriceSeries {
    pub fn is_empty(&self) -> bool {
        !self.close.iter().any(Option::is_some)
    }
}

pub type PriceData = HashMap<String, PriceSeries>;

/// Anything that can supply price history, so the screener can be tested
/// without touching the network.
#[allow(async_fn_in_trait)]
pub trait PriceSource {
    async fn download_price_data(&self, tickers: &[String]) -> anyhow::Result<PriceData>;
}

pub struct TechnicalPriceClient {
    client: reqwest::Client,
}

impl Default for TechnicalPriceClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TechnicalPriceClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("reqwest client builds");
        TechnicalPriceClient { client }
    }

    async fn download_tickers(&self, tickers: &[String]) -> PriceData {
        futures::stream::iter(tickers.iter().cloned())
            .map(|ticker| async move {
                let series = self.download_ticker(&ticker).await;
                (ticker, series)
            })
            .buffer_unordered(DOWNLOAD_CONCURRENCY)
            .filter_map(|(ticker, series)| async move {
                match series {
                    Ok(series) if !series.is_empty() => Some((ticker, series)),
                    Ok(_) => None,
                    Err(error) => {
                        tracing::debug!("技術面價格下載失敗 ticker={} error={}", ticker, error);
                        None
                    }
                }
            })
            .collect()
            .await
    }

    async fn download_ticker(&self, ticker: &str) -> anyhow::Result<PriceSeries> {
        let response = self
            .client
            .get(format!("{CHART_URL}{ticker}"))
            .query(&[("range", DEFAULT_RANGE), ("interval", DEFAULT_INTERVAL)])
            .send()
            .await?
            .error_for_status()?
            .json::<ChartResponse>()
            .await?;
        Ok(response.into_series())
    }

    /// Tickers with no usable close history after a download pass.
    fn find_failed_tickers(price_data: &PriceData, tickers: &[String]) -> Vec<String> {
        tickers
            .iter()
            .filter(|ticker| price_data.get(*ticker).is_none_or(PriceSeries::is_empty))
            .cloned()
            .collect()
    }
}

impl PriceSource for TechnicalPriceClient {
    async fn download_price_data(&self, tickers: &[String]) -> anyhow::Result<PriceData> {
        Ok(download_with_retries(tickers, |batch| async move {
            self.download_tickers(&batch).await
        })
        .await)
    }
}

/// Download every ticker, then re-download whatever came back empty.
async fn download_with_retries<F, Fut>(tickers: &[String], download: F) -> PriceData
where
    F: Fn(Vec<String>) -> Fut,
    Fut: Future<Output = PriceData>,
{
    let mut price_data = download(tickers.to_vec()).await;
    let mut failed_tickers = TechnicalPriceClient::find_failed_tickers(&price_data, tickers);

    for attempt in 1..=DOWNLOAD_RETRY_ATTEMPTS {
        if failed_tickers.is_empty() {
            break;
        }
        tracing::warn!(
            "技術面價格下載有 ticker 失敗，開始 retry attempt={} failed={}",
            attempt,
            failed_tickers.len()
        );
        price_data.extend(download(failed_tickers.clone()).await);
        failed_tickers = TechnicalPriceClient::find_failed_tickers(&price_data, &failed_tickers);
    }

    if !failed_tickers.is_empty() {
        tracing::warn!(
            "技術面價格下載 retry 後仍然失敗 tickers={} sample={:?}",
            failed_tickers.len(),
            &failed_tickers[..failed_tickers.len().min(20)]
        );
    }

    price_data
}

#[derive(Debug, Deserialize)]
struct ChartResponse {
    chart: Chart,
}

#[derive(Debug, Deserialize)]
struct Chart {
    #[serde(default)]
    result: Vec<ChartResult>,
}

#[derive(Debug, Deserialize)]
struct ChartResult {
    indicators: Indicators,
}

#[derive(Debug, Deserialize)]
struct Indicators {
    #[serde(default)]
    quote: Vec<Quote>,
    #[serde(default)]
    adjclose: Vec<AdjClose>,
}

#[derive(Debug, Default, Deserialize)]
struct Quote {
    #[serde(default)]
    close: Vec<Option<f64>>,
    #[serde(default)]
    volume: Vec<Option<f64>>,
}

#[derive(Debug, Default, Deserialize)]
struct AdjClose {
    #[serde(default)]
    adjclose: Vec<Option<f64>>,
}

impl ChartResponse {
    /// `yfinance`'s `auto_adjust=True` replaces the close with the adjusted
    /// close, so prefer `adjclose` and fall back to the raw close.
    fn into_series(self) -> PriceSeries {
        let Some(result) = self.chart.result.into_iter().next() else {
            return PriceSeries::default();
        };
        let quote = result
            .indicators
            .quote
            .into_iter()
            .next()
            .unwrap_or_default();
        let adjusted = result
            .indicators
            .adjclose
            .into_iter()
            .next()
            .unwrap_or_default()
            .adjclose;

        let close = if adjusted.len() == quote.close.len() && !adjusted.is_empty() {
            adjusted
        } else {
            quote.close
        };
        PriceSeries {
            close,
            volume: quote.volume,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_the_adjusted_close() {
        let json = r#"{"chart":{"result":[{"indicators":{
            "quote":[{"close":[1.0,2.0],"volume":[10,20]}],
            "adjclose":[{"adjclose":[0.9,1.9]}]}}]}}"#;

        let series = serde_json::from_str::<ChartResponse>(json)
            .unwrap()
            .into_series();

        assert_eq!(series.close, vec![Some(0.9), Some(1.9)]);
        assert_eq!(series.volume, vec![Some(10.0), Some(20.0)]);
    }

    #[test]
    fn falls_back_to_the_raw_close() {
        let json = r#"{"chart":{"result":[{"indicators":{
            "quote":[{"close":[1.0,2.0],"volume":[10,20]}]}}]}}"#;

        let series = serde_json::from_str::<ChartResponse>(json)
            .unwrap()
            .into_series();

        assert_eq!(series.close, vec![Some(1.0), Some(2.0)]);
    }

    #[test]
    fn returns_an_empty_series_for_an_unknown_ticker() {
        let json = r#"{"chart":{"result":[],"error":null}}"#;

        let series = serde_json::from_str::<ChartResponse>(json)
            .unwrap()
            .into_series();

        assert!(series.is_empty());
    }

    fn series(close: f64) -> PriceSeries {
        PriceSeries {
            close: vec![Some(close)],
            volume: vec![Some(1.0)],
        }
    }

    #[tokio::test]
    async fn retries_only_the_tickers_that_came_back_empty() {
        let calls = std::sync::Mutex::new(Vec::new());

        let price_data =
            download_with_retries(&["AAPL".to_string(), "MSFT".to_string()], |batch| {
                calls.lock().unwrap().push(batch.clone());
                async move {
                    if batch.len() == 2 {
                        [("AAPL".to_string(), series(1.0))].into_iter().collect()
                    } else {
                        [("MSFT".to_string(), series(3.0))].into_iter().collect()
                    }
                }
            })
            .await;

        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                vec!["AAPL".to_string(), "MSFT".to_string()],
                vec!["MSFT".to_string()]
            ]
        );
        assert_eq!(price_data["AAPL"].close, vec![Some(1.0)]);
        assert_eq!(price_data["MSFT"].close, vec![Some(3.0)]);
    }

    #[tokio::test]
    async fn gives_up_after_the_retry_budget() {
        let call_count = std::sync::Mutex::new(0);

        let price_data = download_with_retries(&["AAPL".to_string()], |_| {
            *call_count.lock().unwrap() += 1;
            async { PriceData::new() }
        })
        .await;

        assert_eq!(*call_count.lock().unwrap(), 1 + DOWNLOAD_RETRY_ATTEMPTS);
        assert!(price_data.is_empty());
    }

    #[test]
    fn finds_tickers_without_usable_history() {
        let mut price_data = PriceData::new();
        price_data.insert(
            "AAPL".to_string(),
            PriceSeries {
                close: vec![Some(1.0)],
                volume: vec![Some(1.0)],
            },
        );
        price_data.insert("MSFT".to_string(), PriceSeries::default());

        let failed = TechnicalPriceClient::find_failed_tickers(
            &price_data,
            &["AAPL".to_string(), "MSFT".to_string(), "NVDA".to_string()],
        );

        assert_eq!(failed, ["MSFT", "NVDA"]);
    }
}
