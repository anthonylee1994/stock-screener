//! Builds the screener SELECT and COUNT statements.

use crate::models::SqlValue;
use crate::utils::screener_rules::{
    CHANGE_PERCENT_COLUMN, MARKET_CAP_COLUMN, MIN_VOLUME, POTENTIAL_STOCK_COLUMN,
    QUOTE_CHANGE_PERCENT_COLUMN, SEARCH_COLUMNS, TOTAL_SCORE_COLUMN, VOLUME_COLUMN,
    market_cap_range, normalize_sort_value,
};
use crate::utils::stock_schema::{
    STOCKS_COLUMNS, STOCKS_TABLE, quote_identifier, stocks_select_columns_sql,
};
use crate::utils::ticker_normalizer::normalize_ticker_list;

pub fn search_condition_sql(column: &str, placeholder: &str) -> String {
    format!("LOWER(\"{column}\") LIKE {placeholder} ESCAPE '\\'")
}

/// Accumulates WHERE fragments and their bound parameters.
struct ScreenerFilters {
    placeholder: String,
    where_sql: Vec<String>,
    params: Vec<SqlValue>,
}

impl ScreenerFilters {
    fn new(placeholder: &str) -> Self {
        ScreenerFilters {
            placeholder: placeholder.to_string(),
            // Rows without a total score, or too thin to trade, never surface.
            where_sql: vec![
                format!("{} IS NOT NULL", quote_identifier(TOTAL_SCORE_COLUMN)),
                format!("{} >= {MIN_VOLUME}", quote_identifier(VOLUME_COLUMN)),
            ],
            params: Vec::new(),
        }
    }

    fn add_sector(&mut self, sector: &str) {
        if sector == "All" {
            return;
        }
        self.where_sql
            .push(format!("\"Sector\" = {}", self.placeholder));
        self.params.push(SqlValue::Text(sector.to_string()));
    }

    fn add_market_cap(&mut self, market_cap: &str) {
        let (min_cap, max_cap) = market_cap_range(market_cap);
        if let Some(min_cap) = min_cap {
            self.where_sql.push(format!(
                "{} >= {}",
                quote_identifier(MARKET_CAP_COLUMN),
                self.placeholder
            ));
            self.params.push(SqlValue::Integer(min_cap));
        }
        if let Some(max_cap) = max_cap {
            self.where_sql.push(format!(
                "{} < {}",
                quote_identifier(MARKET_CAP_COLUMN),
                self.placeholder
            ));
            self.params.push(SqlValue::Integer(max_cap));
        }
    }

    fn add_potential_stock(&mut self, potential_stock: bool) {
        if !potential_stock {
            return;
        }
        self.where_sql
            .push(format!("{} = 1", quote_identifier(POTENTIAL_STOCK_COLUMN)));
    }

    /// With no explicit column the search spans every searchable column.
    fn add_search(&mut self, search_column: Option<&str>, like_value: &str) {
        match search_column {
            Some(column) => {
                self.where_sql
                    .push(search_condition_sql(column, &self.placeholder));
                self.params.push(SqlValue::Text(like_value.to_string()));
            }
            None => {
                let conditions: Vec<String> = SEARCH_COLUMNS
                    .iter()
                    .map(|column| search_condition_sql(column, &self.placeholder))
                    .collect();
                self.where_sql
                    .push(format!("({})", conditions.join(" OR ")));
                for _ in SEARCH_COLUMNS {
                    self.params.push(SqlValue::Text(like_value.to_string()));
                }
            }
        }
    }
}

pub struct StockScreenerQueryBuilder {
    pub placeholder: String,
}

impl Default for StockScreenerQueryBuilder {
    fn default() -> Self {
        StockScreenerQueryBuilder {
            placeholder: "?".to_string(),
        }
    }
}

/// A statement and the parameters bound to it.
pub type Query = (String, Vec<SqlValue>);

impl StockScreenerQueryBuilder {
    pub fn new(placeholder: &str) -> Self {
        StockScreenerQueryBuilder {
            placeholder: placeholder.to_string(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_screener_query(
        &self,
        sector: &str,
        market_cap: &str,
        search: &str,
        order: &str,
        ascend: bool,
        limit: i64,
        offset: i64,
        potential_stock: bool,
        search_column: Option<&str>,
    ) -> Query {
        let filters =
            self.build_screener_filters(sector, market_cap, search, potential_stock, search_column);
        self.build_select_query(&filters, order, ascend, limit, offset)
    }

    pub fn build_screener_count_query(
        &self,
        sector: &str,
        market_cap: &str,
        search: &str,
        potential_stock: bool,
        search_column: Option<&str>,
    ) -> Query {
        let filters =
            self.build_screener_filters(sector, market_cap, search, potential_stock, search_column);
        self.build_count_query(&filters.where_sql, filters.params)
    }

    fn build_screener_filters(
        &self,
        sector: &str,
        market_cap: &str,
        search: &str,
        potential_stock: bool,
        search_column: Option<&str>,
    ) -> ScreenerFilters {
        let mut filters = ScreenerFilters::new(&self.placeholder);
        filters.add_sector(sector);
        filters.add_market_cap(market_cap);
        filters.add_potential_stock(potential_stock);

        let normalized_search = search.trim();
        if !normalized_search.is_empty() {
            filters.add_search(search_column, &self.like_contains_value(normalized_search));
        }
        filters
    }

    pub fn build_ticker_screener_query(
        &self,
        tickers: &[String],
        order: &str,
        ascend: bool,
        limit: i64,
        offset: i64,
    ) -> Query {
        let normalized_tickers = normalize_ticker_list(tickers);
        if normalized_tickers.is_empty() {
            return self.empty_select_query();
        }

        let (where_sql, params) = self.build_ticker_filter(&normalized_tickers);
        let filters = ScreenerFilters {
            placeholder: self.placeholder.clone(),
            where_sql,
            params,
        };
        self.build_select_query(&filters, order, ascend, limit, offset)
    }

    pub fn build_ticker_screener_count_query(&self, tickers: &[String]) -> Query {
        let normalized_tickers = normalize_ticker_list(tickers);
        if normalized_tickers.is_empty() {
            return self.empty_count_query();
        }

        let (where_sql, params) = self.build_ticker_filter(&normalized_tickers);
        self.build_count_query(&where_sql, params)
    }

    fn build_ticker_filter(&self, tickers: &[String]) -> (Vec<String>, Vec<SqlValue>) {
        let placeholders = tickers
            .iter()
            .map(|_| self.placeholder.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let where_sql = vec![format!("UPPER(\"Ticker\") IN ({placeholders})")];
        let params = tickers
            .iter()
            .map(|ticker| SqlValue::Text(ticker.clone()))
            .collect();
        (where_sql, params)
    }

    fn build_select_query(
        &self,
        filters: &ScreenerFilters,
        order: &str,
        ascend: bool,
        limit: i64,
        offset: i64,
    ) -> Query {
        let order_column = self.normalize_sort_column(order);
        let direction = if ascend { "ASC" } else { "DESC" };
        let quoted_order = sort_expression_sql(&order_column);
        let query = format!(
            "SELECT {} FROM \"{STOCKS_TABLE}\" WHERE {} \
             ORDER BY {quoted_order} IS NULL, {quoted_order} {direction}, \"Ticker\" ASC \
             LIMIT {} OFFSET {}",
            stocks_select_columns_sql(),
            filters.where_sql.join(" AND "),
            self.placeholder,
            self.placeholder,
        );
        let mut params = filters.params.clone();
        params.push(SqlValue::Integer(limit.max(0)));
        params.push(SqlValue::Integer(offset.max(0)));
        (query, params)
    }

    fn build_count_query(&self, where_sql: &[String], params: Vec<SqlValue>) -> Query {
        let query = format!(
            "SELECT COUNT(*) AS count FROM \"{STOCKS_TABLE}\" WHERE {}",
            where_sql.join(" AND ")
        );
        (query, params)
    }

    fn empty_select_query(&self) -> Query {
        (
            format!(
                "SELECT {} FROM \"{STOCKS_TABLE}\" WHERE 0",
                stocks_select_columns_sql()
            ),
            Vec::new(),
        )
    }

    fn empty_count_query(&self) -> Query {
        ("SELECT 0 AS count".to_string(), Vec::new())
    }

    /// Map an API sort value onto a real column, falling back to market cap.
    pub fn normalize_sort_column(&self, order: &str) -> String {
        let mut column = normalize_sort_value(order);
        if column == "Quote Volume" {
            column = VOLUME_COLUMN.to_string();
        }
        if !is_stocks_column(&column)
            && let Some(trimmed) = column.strip_suffix('.')
        {
            column = trimmed.to_string();
        }
        if !is_stocks_column(&column) {
            return MARKET_CAP_COLUMN.to_string();
        }
        column
    }

    fn like_contains_value(&self, value: &str) -> String {
        format!("%{}%", escape_like_value(&value.to_lowercase()))
    }
}

/// The daily move is served as `Change` with the quote as a fallback, so the
/// sort has to read the same value the API returns, not just the raw column.
fn sort_expression_sql(column: &str) -> String {
    if column == CHANGE_PERCENT_COLUMN {
        return format!(
            "COALESCE({}, {})",
            quote_identifier(CHANGE_PERCENT_COLUMN),
            quote_identifier(QUOTE_CHANGE_PERCENT_COLUMN)
        );
    }
    quote_identifier(column)
}

fn is_stocks_column(column: &str) -> bool {
    STOCKS_COLUMNS.iter().any(|(name, _)| *name == column)
}

fn escape_like_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> SqlValue {
        SqlValue::Text(value.to_string())
    }

    #[test]
    fn includes_filters_search_sort_and_pagination() {
        let builder = StockScreenerQueryBuilder::default();

        let (query, params) = builder.build_screener_query(
            "Technology",
            "Large",
            "a_p%",
            "volume",
            true,
            25,
            50,
            true,
            None,
        );

        assert!(query.contains("\"Sector\" = ?"));
        assert!(query.contains("\"Market Cap\" >= ?"));
        assert!(query.contains("\"Market Cap\" < ?"));
        assert!(query.contains("\"Potential Stock\" = 1"));
        assert!(query.contains("(LOWER(\"Ticker\") LIKE ? ESCAPE"));
        assert!(query.contains("LOWER(\"Company\") LIKE ? ESCAPE"));
        assert!(query.contains("ORDER BY \"Volume\" IS NULL, \"Volume\" ASC, \"Ticker\" ASC"));
        assert_eq!(
            params,
            vec![
                text("Technology"),
                SqlValue::Integer(10_000_000_000),
                SqlValue::Integer(200_000_000_000),
                text("%a\\_p\\%%"),
                text("%a\\_p\\%%"),
                SqlValue::Integer(25),
                SqlValue::Integer(50),
            ]
        );
    }

    #[test]
    fn uses_a_specific_search_column_and_an_open_market_cap() {
        let builder = StockScreenerQueryBuilder::new("%s");

        let (query, params) =
            builder.build_screener_count_query("All", "+Mid", " msft\\ ", false, Some("Company"));

        assert!(!query.contains("\"Sector\" = %s"));
        assert!(query.contains("\"Market Cap\" >= %s"));
        assert!(!query.contains("\"Market Cap\" < %s"));
        assert!(query.contains("LOWER(\"Company\") LIKE %s ESCAPE"));
        assert!(!query.contains("LOWER(\"Ticker\") LIKE %s ESCAPE"));
        assert_eq!(
            params,
            vec![SqlValue::Integer(2_000_000_000), text("%msft\\\\%")]
        );
    }

    #[test]
    fn normalizes_and_ignores_blank_tickers() {
        let builder = StockScreenerQueryBuilder::new("%s");

        let (query, params) = builder.build_ticker_screener_query(
            &[" aapl ".to_string(), String::new(), "msft".to_string()],
            "total_score",
            false,
            -5,
            -1,
        );

        assert!(query.contains("UPPER(\"Ticker\") IN (%s, %s)"));
        assert!(
            query
                .contains("ORDER BY \"Total Score\" IS NULL, \"Total Score\" DESC, \"Ticker\" ASC")
        );
        assert_eq!(
            params,
            vec![
                text("AAPL"),
                text("MSFT"),
                SqlValue::Integer(0),
                SqlValue::Integer(0)
            ]
        );
    }

    #[test]
    fn returns_an_empty_query_without_tickers() {
        let builder = StockScreenerQueryBuilder::default();

        let (query, params) = builder.build_ticker_screener_query(
            &[String::new(), " ".to_string()],
            "market_cap",
            false,
            10,
            0,
        );

        assert!(query.ends_with("WHERE 0"));
        assert!(params.is_empty());

        let (count_query, count_params) = builder.build_ticker_screener_count_query(&[]);

        assert_eq!(count_query, "SELECT 0 AS count");
        assert!(count_params.is_empty());
    }

    #[test]
    fn falls_back_to_market_cap_for_an_unknown_sort() {
        let builder = StockScreenerQueryBuilder::default();

        assert_eq!(builder.normalize_sort_column("Quote Volume"), "Volume");
        assert_eq!(
            builder.normalize_sort_column("target_price_upside"),
            "Target Price Upside"
        );
        assert_eq!(builder.normalize_sort_column("unknown"), "Market Cap");
        assert_eq!(builder.normalize_sort_column("Market Cap."), "Market Cap");
    }

    #[test]
    fn sorts_by_the_target_price_upside_column() {
        let builder = StockScreenerQueryBuilder::default();

        let (query, params) = builder.build_screener_query(
            "All",
            "+Large",
            "",
            "target_price_upside",
            false,
            10,
            0,
            false,
            None,
        );

        assert!(query.contains(
            "ORDER BY \"Target Price Upside\" IS NULL, \
             \"Target Price Upside\" DESC, \"Ticker\" ASC"
        ));
        assert_eq!(
            params,
            vec![
                SqlValue::Integer(10_000_000_000),
                SqlValue::Integer(10),
                SqlValue::Integer(0)
            ]
        );
    }

    #[test]
    fn sorts_the_change_percent_with_the_quote_fallback() {
        let builder = StockScreenerQueryBuilder::default();

        let (query, _) = builder.build_screener_query(
            "All",
            "+Large",
            "",
            "change_percent",
            false,
            10,
            0,
            false,
            None,
        );

        assert!(query.contains(
            "ORDER BY COALESCE(\"Change\", \"Quote Change Percent\") IS NULL, \
             COALESCE(\"Change\", \"Quote Change Percent\") DESC, \"Ticker\" ASC"
        ));
    }

    #[test]
    fn escapes_like_wildcards() {
        assert_eq!(
            search_condition_sql("Ticker", "?"),
            "LOWER(\"Ticker\") LIKE ? ESCAPE '\\'"
        );
    }
}
