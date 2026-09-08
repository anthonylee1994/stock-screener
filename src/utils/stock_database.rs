//! SQLite persistence for the screened stock universe.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use rusqlite::types::{ToSqlOutput, Value};
use rusqlite::{Connection, Row, ToSql};

use crate::models::{SqlValue, StockRow};
use crate::utils::screener_rules::SEARCH_COLUMNS;
use crate::utils::stock_schema::{
    STOCKS_COLUMNS, STOCKS_NEXT_TABLE, STOCKS_TABLE, create_table_sql, ensure_indexes,
    ensure_table, quote_identifier,
};
use crate::utils::stock_screener_query_builder::StockScreenerQueryBuilder;

pub const DEFAULT_DATABASE_PATH: &str = "./data/db.sqlite";

impl ToSql for SqlValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(match self {
            SqlValue::Null => ToSqlOutput::Owned(Value::Null),
            SqlValue::Text(value) => ToSqlOutput::Borrowed(value.as_str().into()),
            SqlValue::Real(value) => ToSqlOutput::Owned(Value::Real(*value)),
            SqlValue::Integer(value) => ToSqlOutput::Owned(Value::Integer(*value)),
        })
    }
}

/// Serializes table swaps so a refresh never races another writer.
fn write_lock() -> &'static Mutex<()> {
    static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    WRITE_LOCK.get_or_init(|| Mutex::new(()))
}

/// Every request to the screener needs the page of rows *and* the total number
/// of matching rows behind it.
pub struct ScreenerPage {
    pub rows: Vec<StockRow>,
    pub total_count: i64,
}

/// Filters for one screener read.
pub struct ScreenerQuery {
    pub limit: i64,
    pub order: String,
    pub ascend: bool,
    pub sector: String,
    pub market_cap: String,
    pub search: String,
    pub tickers: Vec<String>,
    pub offset: i64,
    pub potential_stock: bool,
}

#[derive(Clone)]
pub struct StockDatabase {
    database_path: PathBuf,
    query_builder_placeholder: String,
}

impl Default for StockDatabase {
    fn default() -> Self {
        Self::new(resolve_database_path())
    }
}

pub fn resolve_database_path() -> PathBuf {
    PathBuf::from(
        std::env::var("SQLITE_DB_PATH").unwrap_or_else(|_| DEFAULT_DATABASE_PATH.to_string()),
    )
}

impl StockDatabase {
    pub fn new(database_path: impl AsRef<Path>) -> Self {
        StockDatabase {
            database_path: database_path.as_ref().to_path_buf(),
            query_builder_placeholder: "?".to_string(),
        }
    }

    pub fn display_name(&self) -> String {
        self.database_path.display().to_string()
    }

    fn query_builder(&self) -> StockScreenerQueryBuilder {
        StockScreenerQueryBuilder::new(&self.query_builder_placeholder)
    }

    pub fn connect(&self) -> rusqlite::Result<Connection> {
        if let Some(parent) = self.database_path.parent()
            && !parent.as_os_str().is_empty()
        {
            let _ = std::fs::create_dir_all(parent);
        }
        Connection::open(&self.database_path)
    }

    pub fn initialize(&self) -> rusqlite::Result<()> {
        let connection = self.connect()?;
        ensure_table(&connection, STOCKS_TABLE)?;
        tracing::info!("股票資料庫已準備好 db={}", self.display_name());
        Ok(())
    }

    pub fn has_stocks(&self) -> rusqlite::Result<bool> {
        let connection = self.connect()?;
        let exists: i64 = connection.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM \"{STOCKS_TABLE}\" LIMIT 1)"),
            [],
            |row| row.get(0),
        )?;
        Ok(exists != 0)
    }

    pub fn read_stocks(&self) -> rusqlite::Result<Vec<StockRow>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(&format!("SELECT * FROM \"{STOCKS_TABLE}\""))?;
        let rows = statement
            .query_map([], read_stock_row)?
            .collect::<rusqlite::Result<Vec<StockRow>>>()?;
        Ok(rows)
    }

    pub fn read_screener_stocks_with_count(
        &self,
        query: &ScreenerQuery,
    ) -> rusqlite::Result<ScreenerPage> {
        let connection = self.connect()?;
        let builder = self.query_builder();

        let page = if query.tickers.is_empty() {
            let (search_column, total_count) = self.resolve_search_column_and_count(
                &connection,
                &query.sector,
                &query.market_cap,
                &query.search,
                query.potential_stock,
            )?;
            let (data_sql, data_params) = builder.build_screener_query(
                &query.sector,
                &query.market_cap,
                &query.search,
                &query.order,
                query.ascend,
                query.limit,
                query.offset,
                query.potential_stock,
                search_column.as_deref(),
            );
            ScreenerPage {
                rows: read_rows(&connection, &data_sql, &data_params)?,
                total_count,
            }
        } else {
            let (data_sql, data_params) = builder.build_ticker_screener_query(
                &query.tickers,
                &query.order,
                query.ascend,
                query.limit,
                query.offset,
            );
            let (count_sql, count_params) =
                builder.build_ticker_screener_count_query(&query.tickers);
            ScreenerPage {
                rows: read_rows(&connection, &data_sql, &data_params)?,
                total_count: read_count(&connection, &count_sql, &count_params)?,
            }
        };

        tracing::info!(
            "stocks table SQL screener rows={} total={} limit={} offset={}",
            page.rows.len(),
            page.total_count,
            query.limit,
            query.offset
        );
        Ok(page)
    }

    /// Search one column at a time so a ticker match wins over a company match,
    /// and report how many rows that column found.
    fn resolve_search_column_and_count(
        &self,
        connection: &Connection,
        sector: &str,
        market_cap: &str,
        search: &str,
        potential_stock: bool,
    ) -> rusqlite::Result<(Option<String>, i64)> {
        let builder = self.query_builder();
        let normalized_search = search.trim();
        if normalized_search.is_empty() {
            let (sql, params) =
                builder.build_screener_count_query(sector, market_cap, "", potential_stock, None);
            return Ok((None, read_count(connection, &sql, &params)?));
        }

        let last_column = SEARCH_COLUMNS.last().copied();
        for search_column in SEARCH_COLUMNS {
            let (sql, params) = builder.build_screener_count_query(
                sector,
                market_cap,
                normalized_search,
                potential_stock,
                Some(search_column),
            );
            let total_count = read_count(connection, &sql, &params)?;
            if total_count > 0 || Some(*search_column) == last_column {
                return Ok((Some((*search_column).to_string()), total_count));
            }
        }

        Ok((last_column.map(str::to_string), 0))
    }

    /// Drop rows without a total score, then swap in the rest.
    pub fn replace_scored_stocks(&self, rows: Vec<StockRow>) -> rusqlite::Result<Vec<StockRow>> {
        let total = rows.len();
        let filtered: Vec<StockRow> = rows
            .into_iter()
            .filter(|row| row.total_score.is_some())
            .collect();
        let removed_count = total - filtered.len();
        if removed_count > 0 {
            tracing::info!("已過濾冇 Total Score stocks rows={}", removed_count);
        }
        self.replace_stocks(&filtered)?;
        Ok(filtered)
    }

    /// Build the replacement table first, then swap it in, so readers never see
    /// a half-written universe.
    pub fn replace_stocks(&self, rows: &[StockRow]) -> rusqlite::Result<()> {
        if rows.is_empty() {
            tracing::warn!("股票資料為空；唔會覆蓋 stocks table");
            return Ok(());
        }

        let _guard = write_lock().lock().expect("write lock is not poisoned");
        let mut connection = self.connect()?;
        connection.execute_batch(&format!("DROP TABLE IF EXISTS \"{STOCKS_NEXT_TABLE}\""))?;
        insert_stocks(&mut connection, STOCKS_NEXT_TABLE, rows)?;
        connection.execute_batch(&format!(
            "DROP TABLE IF EXISTS \"{STOCKS_TABLE}\"; \
             ALTER TABLE \"{STOCKS_NEXT_TABLE}\" RENAME TO \"{STOCKS_TABLE}\";"
        ))?;
        ensure_indexes(&connection, STOCKS_TABLE)?;

        tracing::info!("stocks table 已更新 rows={}", rows.len());
        Ok(())
    }
}

fn insert_stocks(
    connection: &mut Connection,
    table_name: &str,
    rows: &[StockRow],
) -> rusqlite::Result<()> {
    connection.execute_batch(&create_table_sql(table_name))?;

    let quoted_columns = STOCKS_COLUMNS
        .iter()
        .map(|(column, _)| quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = vec!["?"; STOCKS_COLUMNS.len()].join(", ");
    let insert_sql =
        format!("INSERT INTO \"{table_name}\" ({quoted_columns}) VALUES ({placeholders})");

    let started_at = Instant::now();
    let transaction = connection.transaction()?;
    {
        let mut statement = transaction.prepare(&insert_sql)?;
        for row in rows {
            let values = row.sql_values();
            statement.execute(rusqlite::params_from_iter(values.iter()))?;
        }
    }
    transaction.commit()?;
    tracing::info!(
        "stocks insert table={} rows={} elapsed_ms={:.2}",
        table_name,
        rows.len(),
        started_at.elapsed().as_secs_f64() * 1000.0
    );
    Ok(())
}

fn read_rows(
    connection: &Connection,
    sql: &str,
    params: &[SqlValue],
) -> rusqlite::Result<Vec<StockRow>> {
    let started_at = Instant::now();
    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(params.iter()), read_stock_row)?
        .collect::<rusqlite::Result<Vec<StockRow>>>()?;
    tracing::debug!(
        "SQL query elapsed_ms={:.2} rows={} sql={}",
        started_at.elapsed().as_secs_f64() * 1000.0,
        rows.len(),
        normalize_sql_for_log(sql)
    );
    Ok(rows)
}

fn read_count(connection: &Connection, sql: &str, params: &[SqlValue]) -> rusqlite::Result<i64> {
    let count = connection.query_row(sql, rusqlite::params_from_iter(params.iter()), |row| {
        row.get::<_, Option<i64>>(0)
    })?;
    Ok(count.unwrap_or(0))
}

pub fn normalize_sql_for_log(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn read_stock_row(row: &Row<'_>) -> rusqlite::Result<StockRow> {
    Ok(StockRow {
        ticker: row.get("Ticker")?,
        company: row.get("Company")?,
        sector: row.get("Sector")?,
        market_cap: row.get("Market Cap")?,
        market_cap_score: row.get("Market Cap Score")?,
        forward_pe: row.get("Forward P/E")?,
        forward_pe_score: row.get("Forward P/E Score")?,
        peg: row.get("PEG")?,
        peg_score: row.get("PEG Score")?,
        ps: row.get("P/S")?,
        ps_score: row.get("P/S Score")?,
        pfcf: row.get("P/FCF")?,
        pfcf_score: row.get("P/FCF Score")?,
        eps_past_5y: row.get("EPS Past 5Y")?,
        eps_past_5y_score: row.get("EPS Past 5Y Score")?,
        sales_past_5y: row.get("Sales Past 5Y")?,
        sales_past_5y_score: row.get("Sales Past 5Y Score")?,
        eps_quarter_over_quarter: row.get("EPS Quarter Over Quarter")?,
        sales_quarter_over_quarter: row.get("Sales Quarter Over Quarter")?,
        roe: row.get("ROE")?,
        roe_score: row.get("ROE Score")?,
        roic: row.get("ROIC")?,
        roic_score: row.get("ROIC Score")?,
        profit_margin: row.get("Profit Margin")?,
        profit_margin_score: row.get("Profit Margin Score")?,
        gross_margin: row.get("Gross Margin")?,
        gross_margin_score: row.get("Gross Margin Score")?,
        operating_margin: row.get("Operating Margin")?,
        debt_equity: row.get("Debt/Equity")?,
        debt_equity_score: row.get("Debt/Equity Score")?,
        short_interest: row.get("Short Interest")?,
        sma200: row.get("200-Day Simple Moving Average")?,
        high_52w: row.get("52W High")?,
        target_price: row.get("Target Price")?,
        target_price_upside: row.get("Target Price Upside")?,
        potential_stock: row
            .get::<_, Option<i64>>("Potential Stock")?
            .map(|value| value != 0),
        price: row.get("Price")?,
        change: row.get("Change")?,
        volume: row.get("Volume")?,
        quote_price: row.get("Quote Price")?,
        quote_change: row.get("Quote Change")?,
        quote_change_percent: row.get("Quote Change Percent")?,
        quote_volume: row.get("Quote Volume")?,
        fundamental_score: row.get("Fundamental Score")?,
        long_term_score: row.get("Long Term Score")?,
        mid_term_score: row.get("Mid Term Score")?,
        short_term_score: row.get("Short Term Score")?,
        technical_score: row.get("Technical Score")?,
        ema200_distance: row.get("EMA200Distance")?,
        roc125: row.get("ROC125")?,
        ema50_distance: row.get("EMA50Distance")?,
        roc20: row.get("ROC20")?,
        ppo_slope3: row.get("PPO Slope3")?,
        rsi14: row.get("RSI14")?,
        total_score: row.get("Total Score")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_database(name: &str) -> StockDatabase {
        let path = std::env::temp_dir()
            .join("stock-screener-tests")
            .join(format!("{name}.sqlite"));
        let _ = std::fs::remove_file(&path);
        StockDatabase::new(path)
    }

    fn stock(
        ticker: &str,
        company: &str,
        sector: &str,
        market_cap: f64,
        total: Option<f64>,
    ) -> StockRow {
        StockRow {
            company: Some(company.to_string()),
            sector: Some(sector.to_string()),
            market_cap: Some(market_cap),
            volume: Some(10_000_000.0),
            total_score: total,
            ..StockRow::with_ticker(ticker)
        }
    }

    fn sample_rows() -> Vec<StockRow> {
        vec![
            stock(
                "AAPL",
                "Apple Inc",
                "Technology",
                3_000_000_000_000.0,
                Some(90.0),
            ),
            stock(
                "MSFT",
                "Microsoft Corp",
                "Technology",
                2_500_000_000_000.0,
                None,
            ),
            StockRow {
                potential_stock: Some(true),
                ..stock(
                    "JPM",
                    "JPMorgan Chase",
                    "Financial",
                    500_000_000_000.0,
                    Some(75.0),
                )
            },
        ]
    }

    fn screener_query(limit: i64) -> ScreenerQuery {
        ScreenerQuery {
            limit,
            order: "Total Score".to_string(),
            ascend: false,
            sector: "All".to_string(),
            market_cap: "+Mid".to_string(),
            search: String::new(),
            tickers: Vec::new(),
            offset: 0,
            potential_stock: false,
        }
    }

    #[test]
    fn replaces_filters_reads_and_counts_stocks() {
        let database = temp_database("replace-read-count");
        database.initialize().unwrap();

        assert!(!database.has_stocks().unwrap());
        database.replace_stocks(&[]).unwrap();

        let filtered = database.replace_scored_stocks(sample_rows()).unwrap();
        let tickers: Vec<&str> = filtered.iter().map(StockRow::ticker_str).collect();
        assert_eq!(tickers, ["AAPL", "JPM"]);
        assert!(database.has_stocks().unwrap());

        let stored = database.read_stocks().unwrap();
        let stored_tickers: Vec<&str> = stored.iter().map(StockRow::ticker_str).collect();
        assert_eq!(stored_tickers, ["AAPL", "JPM"]);
        assert_eq!(stored[0].potential_stock, Some(false));
        assert_eq!(stored[1].potential_stock, Some(true));

        let page = database
            .read_screener_stocks_with_count(&ScreenerQuery {
                tickers: vec!["jpm".to_string(), "aapl".to_string()],
                limit: 10,
                ..screener_query(10)
            })
            .unwrap();
        assert_eq!(page.total_count, 2);
        let page_tickers: Vec<&str> = page.rows.iter().map(StockRow::ticker_str).collect();
        assert_eq!(page_tickers, ["AAPL", "JPM"]);

        let page = database
            .read_screener_stocks_with_count(&ScreenerQuery {
                order: "unknown".to_string(),
                ascend: true,
                sector: "Technology".to_string(),
                market_cap: "Mega".to_string(),
                search: "Apple".to_string(),
                ..screener_query(10)
            })
            .unwrap();
        assert_eq!(page.total_count, 1);
        assert_eq!(page.rows[0].ticker_str(), "AAPL");
    }

    #[test]
    fn resolves_the_search_column_and_its_count() {
        let database = temp_database("search-column");
        database.initialize().unwrap();
        database.replace_scored_stocks(sample_rows()).unwrap();
        let connection = database.connect().unwrap();

        assert_eq!(
            database
                .resolve_search_column_and_count(&connection, "All", "Unknown", "", true)
                .unwrap(),
            (None, 1)
        );
        assert_eq!(
            database
                .resolve_search_column_and_count(&connection, "All", "Unknown", "not-found", false)
                .unwrap(),
            (Some("Company".to_string()), 0)
        );
        assert_eq!(
            database
                .resolve_search_column_and_count(&connection, "All", "Unknown", "jpm", false)
                .unwrap(),
            (Some("Ticker".to_string()), 1)
        );
    }

    #[test]
    fn round_trips_every_column() {
        let database = temp_database("round-trip");
        database.initialize().unwrap();
        let row = StockRow {
            roic: Some(0.42),
            rsi14: Some(55.5),
            potential_stock: Some(true),
            total_score: Some(88.0),
            volume: Some(2_000_000.0),
            ..stock("NVDA", "NVIDIA Corp", "Technology", 1.0, Some(88.0))
        };

        database.replace_stocks(std::slice::from_ref(&row)).unwrap();
        let stored = database.read_stocks().unwrap();

        assert_eq!(stored, vec![row]);
    }

    #[test]
    fn collapses_whitespace_for_sql_logging() {
        assert_eq!(
            normalize_sql_for_log(" SELECT   *\nFROM items "),
            "SELECT * FROM items"
        );
    }
}
