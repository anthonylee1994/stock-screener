//! Shared column names, market-cap buckets and API-value normalization.

pub const MARKET_CAP_COLUMN: &str = "Market Cap";
pub const TOTAL_SCORE_COLUMN: &str = "Total Score";
pub const FUNDAMENTAL_SCORE_COLUMN: &str = "Fundamental Score";
pub const TECHNICAL_SCORE_COLUMN: &str = "Technical Score";
pub const CHANGE_PERCENT_COLUMN: &str = "Change";
pub const QUOTE_CHANGE_PERCENT_COLUMN: &str = "Quote Change Percent";
pub const VOLUME_COLUMN: &str = "Volume";
pub const POTENTIAL_STOCK_COLUMN: &str = "Potential Stock";
pub const TARGET_PRICE_UPSIDE_COLUMN: &str = "Target Price Upside";
pub const MIN_VOLUME: i64 = 1_000_000;

/// `(minimum, maximum)` market cap in USD. `None` means the side is open.
pub const MARKET_CAP_RANGES: &[(&str, Option<i64>, Option<i64>)] = &[
    ("+Mid", Some(2_000_000_000), None),
    ("+Large", Some(10_000_000_000), None),
    ("Micro", Some(50_000_000), Some(300_000_000)),
    ("Small", Some(300_000_000), Some(2_000_000_000)),
    ("Mid", Some(2_000_000_000), Some(10_000_000_000)),
    ("Large", Some(10_000_000_000), Some(200_000_000_000)),
    ("Mega", Some(200_000_000_000), None),
];

const MARKET_CAP_BY_API_VALUE: &[(&str, &str)] = &[
    ("+mid", "+Mid"),
    ("+large", "+Large"),
    ("micro", "Micro"),
    ("small", "Small"),
    ("mid", "Mid"),
    ("large", "Large"),
    ("mega", "Mega"),
];

pub const SORT_COLUMN_BY_VALUE: &[(&str, &str)] = &[
    ("market_cap", MARKET_CAP_COLUMN),
    ("fundamental_score", FUNDAMENTAL_SCORE_COLUMN),
    ("technical_score", TECHNICAL_SCORE_COLUMN),
    ("total_score", TOTAL_SCORE_COLUMN),
    ("change_percent", CHANGE_PERCENT_COLUMN),
    ("volume", VOLUME_COLUMN),
    ("target_price_upside", TARGET_PRICE_UPSIDE_COLUMN),
];

pub const SEARCH_COLUMNS: &[&str] = &["Ticker", "Company"];

pub fn market_cap_range(market_cap: &str) -> (Option<i64>, Option<i64>) {
    for (key, min_cap, max_cap) in MARKET_CAP_RANGES {
        if *key == market_cap {
            return (*min_cap, *max_cap);
        }
    }
    (None, None)
}

pub fn normalize_market_cap_value(value: &str) -> String {
    let lowered = value.to_lowercase();
    for (api_value, column) in MARKET_CAP_BY_API_VALUE {
        if *api_value == lowered {
            return (*column).to_string();
        }
    }
    value.to_string()
}

pub fn normalize_sort_value(value: &str) -> String {
    let lowered = value.to_lowercase();
    for (api_value, column) in SORT_COLUMN_BY_VALUE {
        if *api_value == lowered {
            return (*column).to_string();
        }
    }
    value.to_string()
}
