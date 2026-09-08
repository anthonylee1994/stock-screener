//! Finviz screener HTTP client, replacing `finvizfinance`'s network layer.

use std::time::Duration;

use crate::services::fundamental::finviz_custom_screener::{FinvizRow, parse_screener_page};

const SCREENER_URL: &str = "https://finviz.com/screener.ashx";
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_4) \
AppleWebKit/537.36 (KHTML, like Gecko) Chrome/81.0.4044.138 Safari/537.36";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Finviz serves 20 rows per screener page.
const PAGE_SIZE: i64 = 20;
const SLEEP_BETWEEN_PAGES: Duration = Duration::from_millis(200);

/// `v=151` is the custom screener view.
const CUSTOM_VIEW: &str = "151";
/// `Market Cap. = +Mid (over $2bln)` and `Average Volume = Over 1M`.
const FUNDAMENTAL_FILTERS: &str = "cap_midover,sh_avgvol_o1000";
/// `Market Cap.` descending.
const DEFAULT_ORDER: &str = "-marketcap";
/// Finviz custom-screener column indices. Index `0` is always prepended.
pub const CUSTOM_COLUMNS: &[u32] = &[
    1, 2, 3, 6, 8, 9, 10, 13, 19, 21, 22, 23, 30, 33, 34, 39, 38, 40, 41, 54, 57, 69, 65, 66, 67,
];

pub struct FundamentalScreenerClient {
    client: reqwest::Client,
}

impl Default for FundamentalScreenerClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FundamentalScreenerClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("reqwest client builds");
        FundamentalScreenerClient { client }
    }

    /// Walk the screener pages until `limit` rows have been collected.
    pub async fn fetch(&self, limit: i64) -> anyhow::Result<Vec<FinvizRow>> {
        let mut remaining = limit;
        let html = self.fetch_page(None).await?;
        let Some(first_page) = parse_screener_page(&html, None, remaining) else {
            tracing::warn!("Finviz 搵唔到 ticker");
            return Ok(Vec::new());
        };

        let headers = first_page.headers;
        let page_count = first_page.page_count;
        let mut rows = first_page.rows;
        remaining -= PAGE_SIZE;

        for page_index in 1..page_count {
            if remaining <= 0 {
                break;
            }
            tokio::time::sleep(SLEEP_BETWEEN_PAGES).await;
            let offset = page_index as i64 * PAGE_SIZE + 1;
            let html = match self.fetch_page(Some(offset)).await {
                Ok(html) => html,
                Err(error) => {
                    tracing::warn!("Finviz 第 {} 頁下載失敗: {}", page_index + 1, error);
                    break;
                }
            };
            if let Some(page) = parse_screener_page(&html, Some(&headers), remaining) {
                rows.extend(page.rows);
            }
            remaining -= PAGE_SIZE;
            tracing::info!(
                "Finviz 已下載 {}/{} 頁 rows={}",
                page_index + 1,
                page_count,
                rows.len()
            );
        }

        Ok(rows)
    }

    async fn fetch_page(&self, offset: Option<i64>) -> anyhow::Result<String> {
        let mut request = self
            .client
            .get(SCREENER_URL)
            .query(&[("v", CUSTOM_VIEW)])
            .query(&[("f", FUNDAMENTAL_FILTERS)])
            .query(&[("o", DEFAULT_ORDER)])
            .query(&[("c", custom_columns_param().as_str())]);
        if let Some(offset) = offset {
            request = request.query(&[("r", offset.to_string())]);
        }
        let response = request.send().await?.error_for_status()?;
        Ok(response.text().await?)
    }
}

/// Finviz always wants column `0` first, followed by the configured columns.
pub fn custom_columns_param() -> String {
    let mut columns = vec!["0".to_string()];
    columns.extend(
        CUSTOM_COLUMNS
            .iter()
            .filter(|column| **column != 0)
            .map(u32::to_string),
    );
    columns.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepends_column_zero_to_the_custom_columns() {
        let param = custom_columns_param();

        assert!(param.starts_with("0,1,2,3,6,"));
        assert!(param.ends_with(",69,65,66,67"));
    }

    #[test]
    fn requests_the_potential_stock_columns() {
        for column in [23, 30, 39, 40, 54, 57, 69] {
            assert!(CUSTOM_COLUMNS.contains(&column), "缺少 column {column}");
        }
    }
}
