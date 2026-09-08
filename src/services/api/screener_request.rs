//! Reads and bounds the screener query parameters.

use serde_json::{Map, Value};

use crate::utils::screener_rules::{normalize_market_cap_value, normalize_sort_value};
use crate::utils::ticker_normalizer::{TickerInput, normalize_tickers};

pub type Payload = Map<String, Value>;

const DEFAULT_SECTOR: &str = "All";
const DEFAULT_MARKET_CAP: &str = "+large";
const DEFAULT_ORDER: &str = "total_score";
const DEFAULT_ASCEND: bool = false;
const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 100;
const DEFAULT_OFFSET: i64 = 0;

pub struct ScreenerRequest<'a> {
    payload: &'a Payload,
}

impl<'a> ScreenerRequest<'a> {
    pub fn new(payload: &'a Payload) -> Self {
        ScreenerRequest { payload }
    }

    pub fn sector(&self) -> String {
        self.text("sector")
            .unwrap_or_else(|| DEFAULT_SECTOR.to_string())
    }

    pub fn market_cap(&self) -> String {
        let value = self
            .text("market_cap")
            .unwrap_or_else(|| DEFAULT_MARKET_CAP.to_string());
        normalize_market_cap_value(&value)
    }

    pub fn order(&self) -> String {
        let value = self
            .text("order")
            .unwrap_or_else(|| DEFAULT_ORDER.to_string());
        normalize_sort_value(&value)
    }

    pub fn ascend(&self) -> bool {
        parse_bool(self.payload.get("ascend"), DEFAULT_ASCEND)
    }

    pub fn search(&self) -> String {
        self.text("search").unwrap_or_default()
    }

    pub fn tickers(&self) -> Vec<String> {
        normalize_tickers(match self.payload.get("tickers") {
            Some(Value::String(text)) => TickerInput::Text(text.clone()),
            Some(Value::Array(items)) => {
                TickerInput::List(items.iter().map(value_to_string).collect())
            }
            _ => TickerInput::None,
        })
    }

    pub fn potential_stock(&self) -> bool {
        parse_bool(self.payload.get("potential_stock"), false)
    }

    pub fn limit(&self) -> i64 {
        parse_int(self.payload.get("limit"), DEFAULT_LIMIT, 1, Some(MAX_LIMIT))
    }

    pub fn offset(&self) -> i64 {
        parse_int(self.payload.get("offset"), DEFAULT_OFFSET, 0, None)
    }

    fn text(&self, key: &str) -> Option<String> {
        match self.payload.get(key) {
            None | Some(Value::Null) => None,
            Some(value) => Some(value_to_string(value)),
        }
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Anything in the usual truthy set counts as true; anything else is false.
pub fn parse_bool(value: Option<&Value>, default: bool) -> bool {
    match value {
        None | Some(Value::Null) => default,
        Some(Value::Bool(value)) => *value,
        Some(value) => matches!(
            value_to_string(value).to_lowercase().as_str(),
            "1" | "true" | "yes" | "y" | "on"
        ),
    }
}

/// Falls back to `default` when the value is missing or not an integer, then
/// clamps into range.
pub fn parse_int(value: Option<&Value>, default: i64, minimum: i64, maximum: Option<i64>) -> i64 {
    let parsed = match value {
        None | Some(Value::Null) => None,
        Some(Value::Bool(value)) => Some(i64::from(*value)),
        Some(Value::Number(number)) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|value| value.trunc() as i64)),
        Some(Value::String(text)) => text.trim().parse::<i64>().ok(),
        Some(_) => None,
    };

    let mut parsed = parsed.unwrap_or(default).max(minimum);
    if let Some(maximum) = maximum {
        parsed = parsed.min(maximum);
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::screener_rules::{
        MARKET_CAP_COLUMN, TARGET_PRICE_UPSIDE_COLUMN, TOTAL_SCORE_COLUMN,
    };
    use serde_json::json;

    fn payload(value: Value) -> Payload {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn normalizes_api_values_and_bounds_numbers() {
        let payload = payload(json!({
            "market_cap": "large",
            "order": "total_score",
            "ascend": "yes",
            "limit": "250",
            "offset": "-10",
            "tickers": " aapl, MSFT, aapl, , nvda ",
            "potential_stock": "on",
        }));
        let request = ScreenerRequest::new(&payload);

        assert_eq!(request.market_cap(), "Large");
        assert_eq!(request.order(), TOTAL_SCORE_COLUMN);
        assert!(request.ascend());
        assert_eq!(request.limit(), 100);
        assert_eq!(request.offset(), 0);
        assert_eq!(request.tickers(), ["AAPL", "MSFT", "NVDA"]);
        assert!(request.potential_stock());
    }

    #[test]
    fn uses_defaults_for_invalid_values() {
        let payload = payload(json!({"limit": "bad", "offset": "bad"}));
        let request = ScreenerRequest::new(&payload);

        assert_eq!(request.sector(), "All");
        assert_eq!(request.market_cap(), "+Large");
        assert_eq!(request.order(), TOTAL_SCORE_COLUMN);
        assert!(!request.ascend());
        assert_eq!(request.limit(), 100);
        assert_eq!(request.offset(), 0);
        assert!(request.tickers().is_empty());
    }

    #[test]
    fn maps_sort_aliases() {
        let market_cap = payload(json!({"order": "market_cap"}));
        let upside = payload(json!({"order": "target_price_upside"}));

        assert_eq!(ScreenerRequest::new(&market_cap).order(), MARKET_CAP_COLUMN);
        assert_eq!(
            ScreenerRequest::new(&upside).order(),
            TARGET_PRICE_UPSIDE_COLUMN
        );
    }

    #[test]
    fn applies_the_minimum_without_a_maximum() {
        assert_eq!(parse_int(Some(&json!("5")), 0, 10, None), 10);
    }

    #[test]
    fn returns_the_default_for_a_missing_bool() {
        assert!(parse_bool(None, true));
        assert!(parse_bool(Some(&json!(true)), false));
        assert!(!parse_bool(Some(&json!(false)), true));
        assert!(!parse_bool(Some(&json!("nope")), true));
    }

    #[test]
    fn reads_tickers_from_a_json_list() {
        let payload = payload(json!({"tickers": [" nvda ", "NVDA", "msft"]}));

        assert_eq!(ScreenerRequest::new(&payload).tickers(), ["NVDA", "MSFT"]);
    }
}
