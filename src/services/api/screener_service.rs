//! Turns an API payload into a screener response.

use serde_json::Value;

use crate::services::api::screener_request::{Payload, ScreenerRequest};
use crate::services::api::screener_response_formatter::format_response;
use crate::utils::stock_database::{ScreenerPage, ScreenerQuery, StockDatabase};

/// The read side of the stock table, so the service can be tested without
/// touching SQLite.
pub trait ScreenerDatabase {
    fn read_screener_stocks_with_count(
        &self,
        query: &ScreenerQuery,
    ) -> rusqlite::Result<ScreenerPage>;
}

impl ScreenerDatabase for StockDatabase {
    fn read_screener_stocks_with_count(
        &self,
        query: &ScreenerQuery,
    ) -> rusqlite::Result<ScreenerPage> {
        StockDatabase::read_screener_stocks_with_count(self, query)
    }
}

pub struct ScreenerService<D: ScreenerDatabase> {
    database: D,
    api_token: Option<String>,
}

impl Default for ScreenerService<StockDatabase> {
    fn default() -> Self {
        Self::new(StockDatabase::default(), std::env::var("API_TOKEN").ok())
    }
}

impl<D: ScreenerDatabase> ScreenerService<D> {
    pub fn new(database: D, api_token: Option<String>) -> Self {
        ScreenerService {
            database,
            api_token,
        }
    }

    pub fn is_authorized(&self, payload: &Payload) -> bool {
        let supplied = payload.get("api_token").and_then(Value::as_str);
        supplied == self.api_token.as_deref()
    }

    /// Reads one row more than the page size so `has_more` needs no extra query.
    pub fn get_screener_response(&self, payload: &Payload) -> rusqlite::Result<Value> {
        let request = ScreenerRequest::new(payload);
        let limit = request.limit();
        let offset = request.offset();

        let page = self
            .database
            .read_screener_stocks_with_count(&ScreenerQuery {
                limit: limit + 1,
                order: request.order(),
                ascend: request.ascend(),
                sector: request.sector(),
                market_cap: request.market_cap(),
                search: request.search(),
                tickers: request.tickers(),
                offset,
                potential_stock: request.potential_stock(),
            })?;

        let has_more = page.rows.len() as i64 > limit;
        let page_rows = &page.rows[..page.rows.len().min(limit.max(0) as usize)];
        Ok(format_response(
            page_rows,
            Some(page.total_count),
            Some(limit),
            offset,
            has_more,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StockRow;
    use crate::utils::screener_rules::TOTAL_SCORE_COLUMN;
    use serde_json::json;
    use std::sync::Mutex;

    struct FakeDatabase {
        rows: Vec<StockRow>,
        total_count: i64,
        calls: Mutex<Vec<String>>,
    }

    impl ScreenerDatabase for FakeDatabase {
        fn read_screener_stocks_with_count(
            &self,
            query: &ScreenerQuery,
        ) -> rusqlite::Result<ScreenerPage> {
            self.calls.lock().unwrap().push(format!(
                "limit={} sector={} market_cap={} search={} tickers={:?} order={} \
                 ascend={} offset={} potential_stock={}",
                query.limit,
                query.sector,
                query.market_cap,
                query.search,
                query.tickers,
                query.order,
                query.ascend,
                query.offset,
                query.potential_stock,
            ));
            Ok(ScreenerPage {
                rows: self.rows.clone(),
                total_count: self.total_count,
            })
        }
    }

    fn payload(value: Value) -> Payload {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn checks_the_api_token() {
        let service = ScreenerService::new(
            FakeDatabase {
                rows: Vec::new(),
                total_count: 0,
                calls: Mutex::new(Vec::new()),
            },
            Some("secret".to_string()),
        );

        assert!(service.is_authorized(&payload(json!({"api_token": "secret"}))));
        assert!(!service.is_authorized(&payload(json!({"api_token": "wrong"}))));
        assert!(!service.is_authorized(&payload(json!({}))));
    }

    #[test]
    fn requests_one_extra_row_and_formats_the_page() {
        let service = ScreenerService::new(
            FakeDatabase {
                rows: vec![
                    StockRow::with_ticker("AAPL"),
                    StockRow::with_ticker("MSFT"),
                    StockRow::with_ticker("NVDA"),
                ],
                total_count: 5,
                calls: Mutex::new(Vec::new()),
            },
            None,
        );

        let response = service
            .get_screener_response(&payload(json!({
                "sector": "Technology",
                "market_cap": "large",
                "search": "aapl",
                "tickers": "aapl,msft",
                "order": "total_score",
                "ascend": "true",
                "limit": 2,
                "offset": 4,
                "potential_stock": "true",
            })))
            .unwrap();

        assert_eq!(
            *service.database.calls.lock().unwrap(),
            vec![format!(
                "limit=3 sector=Technology market_cap=Large search=aapl \
                 tickers=[\"AAPL\", \"MSFT\"] order={TOTAL_SCORE_COLUMN} \
                 ascend=true offset=4 potential_stock=true"
            )]
        );
        assert_eq!(response["count"], 5);
        assert_eq!(response["limit"], 2);
        assert_eq!(response["offset"], 4);
        assert_eq!(response["has_more"], true);
        assert_eq!(response["next_offset"], 6);
        let tickers: Vec<&str> = response["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["ticker"].as_str().unwrap())
            .collect();
        assert_eq!(tickers, ["AAPL", "MSFT"]);
    }
}
