//! Ticker cleanup shared by the API request layer and the technical screener.

use std::collections::HashSet;

/// Ticker input as it arrives from an API payload: either a comma separated
/// string, a JSON list, or nothing at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickerInput {
    None,
    Text(String),
    List(Vec<String>),
}

impl TickerInput {
    fn into_parts(self) -> Vec<String> {
        match self {
            TickerInput::None => Vec::new(),
            TickerInput::Text(text) => text.split(',').map(str::to_string).collect(),
            TickerInput::List(items) => items,
        }
    }
}

pub fn normalize_tickers(tickers: TickerInput) -> Vec<String> {
    let mut normalized_tickers = Vec::new();
    let mut seen_tickers = HashSet::new();
    for ticker in tickers.into_parts() {
        let normalized_ticker = ticker.trim().to_uppercase();
        if normalized_ticker.is_empty() || seen_tickers.contains(&normalized_ticker) {
            continue;
        }
        seen_tickers.insert(normalized_ticker.clone());
        normalized_tickers.push(normalized_ticker);
    }
    normalized_tickers
}

pub fn normalize_ticker_list<S: AsRef<str>>(tickers: &[S]) -> Vec<String> {
    normalize_tickers(TickerInput::List(
        tickers
            .iter()
            .map(|item| item.as_ref().to_string())
            .collect(),
    ))
}
