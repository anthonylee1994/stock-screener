//! SQLite schema for the persisted stock universe.

use std::collections::HashSet;

use rusqlite::Connection;

use crate::utils::screener_rules::{
    MIN_VOLUME, POTENTIAL_STOCK_COLUMN, SORT_COLUMN_BY_VALUE, TOTAL_SCORE_COLUMN, VOLUME_COLUMN,
};

pub const STOCKS_TABLE: &str = "stocks";
pub const STOCKS_NEXT_TABLE: &str = "stocks_next";

/// Column name and SQLite type, in the same order as
/// [`crate::models::StockRow::sql_values`].
pub const STOCKS_COLUMNS: &[(&str, &str)] = &[
    ("Ticker", "TEXT"),
    ("Company", "TEXT"),
    ("Sector", "TEXT"),
    ("Market Cap", "REAL"),
    ("Market Cap Score", "REAL"),
    ("Forward P/E", "REAL"),
    ("Forward P/E Score", "REAL"),
    ("PEG", "REAL"),
    ("PEG Score", "REAL"),
    ("P/S", "REAL"),
    ("P/S Score", "REAL"),
    ("P/FCF", "REAL"),
    ("P/FCF Score", "REAL"),
    ("EPS Past 5Y", "REAL"),
    ("EPS Past 5Y Score", "REAL"),
    ("Sales Past 5Y", "REAL"),
    ("Sales Past 5Y Score", "REAL"),
    ("EPS Quarter Over Quarter", "REAL"),
    ("Sales Quarter Over Quarter", "REAL"),
    ("ROE", "REAL"),
    ("ROE Score", "REAL"),
    ("ROIC", "REAL"),
    ("ROIC Score", "REAL"),
    ("Profit Margin", "REAL"),
    ("Profit Margin Score", "REAL"),
    ("Gross Margin", "REAL"),
    ("Gross Margin Score", "REAL"),
    ("Operating Margin", "REAL"),
    ("Debt/Equity", "REAL"),
    ("Debt/Equity Score", "REAL"),
    ("Short Interest", "REAL"),
    ("200-Day Simple Moving Average", "REAL"),
    ("52W High", "REAL"),
    ("Target Price", "REAL"),
    ("Target Price Upside", "REAL"),
    ("Potential Stock", "INTEGER"),
    ("Price", "REAL"),
    ("Change", "REAL"),
    ("Volume", "REAL"),
    ("Quote Price", "REAL"),
    ("Quote Change", "REAL"),
    ("Quote Change Percent", "REAL"),
    ("Quote Volume", "REAL"),
    ("Fundamental Score", "REAL"),
    ("Long Term Score", "REAL"),
    ("Mid Term Score", "REAL"),
    ("Short Term Score", "REAL"),
    ("Technical Score", "REAL"),
    ("EMA200Distance", "REAL"),
    ("ROC125", "REAL"),
    ("EMA50Distance", "REAL"),
    ("ROC20", "REAL"),
    ("PPO Slope3", "REAL"),
    ("RSI14", "REAL"),
    ("Total Score", "REAL"),
];

pub fn quote_identifier(value: &str) -> String {
    format!("\"{value}\"")
}

pub fn stocks_select_columns_sql() -> String {
    STOCKS_COLUMNS
        .iter()
        .map(|(column, _)| quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn create_table_sql(table_name: &str) -> String {
    let columns_sql = STOCKS_COLUMNS
        .iter()
        .map(|(column, column_type)| format!("{} {column_type}", quote_identifier(column)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("CREATE TABLE IF NOT EXISTS \"{table_name}\" ({columns_sql})")
}

pub fn ensure_table(connection: &Connection, table_name: &str) -> rusqlite::Result<()> {
    connection.execute_batch(&create_table_sql(table_name))?;
    add_missing_columns(connection, table_name)?;
    ensure_indexes(connection, table_name)
}

pub fn ensure_indexes(connection: &Connection, table_name: &str) -> rusqlite::Result<()> {
    for (index_key, column) in index_columns() {
        connection.execute_batch(&format!(
            "CREATE INDEX IF NOT EXISTS \"{table_name}_{index_key}_idx\" \
             ON \"{table_name}\" ({})",
            quote_identifier(column)
        ))?;
    }
    // Partial indexes matching the screener's standing filters, so the hot read
    // path never scans rows the API would discard anyway.
    for (index_key, column) in SORT_COLUMN_BY_VALUE {
        connection.execute_batch(&format!(
            "CREATE INDEX IF NOT EXISTS \"{table_name}_screener_{index_key}_desc_idx\" \
             ON \"{table_name}\" ({} DESC) \
             WHERE {} IS NOT NULL AND {} >= {MIN_VOLUME}",
            quote_identifier(column),
            quote_identifier(TOTAL_SCORE_COLUMN),
            quote_identifier(VOLUME_COLUMN),
        ))?;
    }
    Ok(())
}

fn index_columns() -> Vec<(&'static str, &'static str)> {
    let mut columns = vec![("sector", "Sector"), ("ticker", "Ticker")];
    columns.extend(SORT_COLUMN_BY_VALUE.iter().copied());
    columns.push(("potential_stock", POTENTIAL_STOCK_COLUMN));
    columns
}

pub fn add_missing_columns(connection: &Connection, table_name: &str) -> rusqlite::Result<()> {
    let existing_columns = fetch_table_columns(connection, table_name)?;
    for (column, column_type) in STOCKS_COLUMNS {
        if !existing_columns.contains(*column) {
            connection.execute_batch(&format!(
                "ALTER TABLE \"{table_name}\" ADD COLUMN {} {column_type}",
                quote_identifier(column)
            ))?;
        }
    }
    Ok(())
}

pub fn fetch_table_columns(
    connection: &Connection,
    table_name: &str,
) -> rusqlite::Result<HashSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info(\"{table_name}\")"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<HashSet<String>>>()?;
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StockRow;

    #[test]
    fn column_list_matches_the_row_layout() {
        assert_eq!(STOCKS_COLUMNS.len(), StockRow::default().sql_values().len());
    }

    #[test]
    fn builds_a_quoted_create_statement() {
        let sql = create_table_sql("stocks");

        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS \"stocks\" (\"Ticker\" TEXT, "));
        assert!(sql.ends_with("\"Total Score\" REAL)"));
    }

    #[test]
    fn adds_missing_columns_to_an_older_table() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE \"stocks\" (\"Ticker\" TEXT)")
            .unwrap();

        ensure_table(&connection, STOCKS_TABLE).unwrap();
        let columns = fetch_table_columns(&connection, STOCKS_TABLE).unwrap();

        assert!(columns.contains("Company"));
        assert_eq!(columns.len(), STOCKS_COLUMNS.len());
    }
}
