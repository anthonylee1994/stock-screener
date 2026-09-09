//! Turn raw Finviz rows into [`StockRow`]s: rename headers, coerce numbers,
//! rescale percentages and round the non-score metrics.

use std::collections::HashMap;

use crate::models::StockRow;
use crate::services::common::series_normalizer::round_option;
use crate::services::fundamental::finviz_custom_screener::{FinvizCell, FinvizRow};

/// Finviz header -> canonical column name.
const COLUMN_ALIASES: &[(&str, &str)] = &[
    ("Market Cap.", "Market Cap"),
    ("Fwd P/E", "Forward P/E"),
    ("P/Free Cash Flow", "P/FCF"),
    ("P/FCF", "P/FCF"),
    ("EPS growth past 5 years", "EPS Past 5Y"),
    ("EPS past 5Y", "EPS Past 5Y"),
    ("EPS Q/Q", "EPS Quarter Over Quarter"),
    ("Sales growth past 5 years", "Sales Past 5Y"),
    ("Sales past 5Y", "Sales Past 5Y"),
    ("Sales Q/Q", "Sales Quarter Over Quarter"),
    ("Return on Equity", "ROE"),
    ("Return on Investments", "ROIC"),
    ("ROI", "ROIC"),
    ("Total Debt/Equity", "Debt/Equity"),
    ("Debt/Eq", "Debt/Equity"),
    ("Net Profit Margin", "Profit Margin"),
    ("Profit M", "Profit Margin"),
    ("Gross M", "Gross Margin"),
    ("Oper M", "Operating Margin"),
    ("Float Short", "Short Interest"),
    ("Short Float", "Short Interest"),
    ("SMA200", "200-Day Simple Moving Average"),
    // Finviz renamed the daily move column, and it arrives as a ratio because
    // `number_covert` already divides the `%` away.
    ("Change %", "Change"),
];

pub fn canonical_column(header: &str) -> &str {
    for (alias, canonical) in COLUMN_ALIASES {
        if *alias == header {
            return canonical;
        }
    }
    header
}

/// Rows keyed by canonical column name.
type CanonicalRow = HashMap<String, FinvizCell>;

#[derive(Default)]
pub struct FundamentalDataNormalizer;

impl FundamentalDataNormalizer {
    /// Drop rows whose ticker is blank or missing.
    pub fn remove_invalid_rows(&self, rows: Vec<FinvizRow>) -> Vec<FinvizRow> {
        if rows.is_empty() {
            return rows;
        }
        // A missing Ticker header means Finviz never returned that column, so
        // there is nothing to filter on and every row is kept.
        if !rows.iter().any(|row| row.contains_key("Ticker")) {
            return rows;
        }
        rows.into_iter()
            .filter(|row| {
                row.get("Ticker")
                    .and_then(FinvizCell::text)
                    .is_some_and(|ticker| !ticker.trim().is_empty())
            })
            .collect()
    }

    pub fn normalize(&self, rows: Vec<FinvizRow>) -> Vec<StockRow> {
        rows.into_iter()
            .map(|row| self.normalize_row(&to_canonical_row(row)))
            .collect()
    }

    fn normalize_row(&self, row: &CanonicalRow) -> StockRow {
        let price = numeric(row, "Price");
        let target_price = round_option(numeric(row, "Target Price"), 4);
        let target_price_upside = target_price_upside(row, target_price, price);

        StockRow {
            ticker: text(row, "Ticker"),
            company: text(row, "Company"),
            sector: text(row, "Sector"),
            market_cap: round_option(numeric(row, "Market Cap"), 4),
            forward_pe: round_option(numeric(row, "Forward P/E"), 4),
            peg: round_option(numeric(row, "PEG"), 4),
            ps: round_option(numeric(row, "P/S"), 4),
            pfcf: round_option(numeric(row, "P/FCF"), 4),
            eps_past_5y: round_option(percent(row, "EPS Past 5Y"), 4),
            sales_past_5y: round_option(percent(row, "Sales Past 5Y"), 4),
            eps_quarter_over_quarter: round_option(percent(row, "EPS Quarter Over Quarter"), 4),
            sales_quarter_over_quarter: round_option(percent(row, "Sales Quarter Over Quarter"), 4),
            roe: round_option(percent(row, "ROE"), 4),
            roic: round_option(percent(row, "ROIC"), 4),
            profit_margin: round_option(percent(row, "Profit Margin"), 4),
            gross_margin: round_option(percent(row, "Gross Margin"), 4),
            operating_margin: round_option(percent(row, "Operating Margin"), 4),
            debt_equity: round_option(numeric(row, "Debt/Equity"), 4),
            short_interest: round_option(percent(row, "Short Interest"), 4),
            sma200: round_option(percent(row, "200-Day Simple Moving Average"), 4),
            high_52w: round_option(percent(row, "52W High"), 4),
            target_price,
            target_price_upside: round_option(target_price_upside, 4),
            price,
            change: numeric(row, "Change"),
            volume: numeric(row, "Volume"),
            ..StockRow::default()
        }
    }
}

fn to_canonical_row(row: FinvizRow) -> CanonicalRow {
    row.into_iter()
        .map(|(header, cell)| (canonical_column(&header).to_string(), cell))
        .collect()
}

fn text(row: &CanonicalRow, column: &str) -> Option<String> {
    row.get(column)
        .and_then(FinvizCell::text)
        .map(str::to_string)
}

fn numeric(row: &CanonicalRow, column: &str) -> Option<f64> {
    row.get(column).and_then(FinvizCell::numeric)
}

/// Percent columns arrive either already scaled (Finviz numeric columns) or as
/// raw `12.5%` text, which still needs the division.
fn percent(row: &CanonicalRow, column: &str) -> Option<f64> {
    let cell = row.get(column)?;
    let value = cell.numeric()?;
    if cell.has_percent_symbol() {
        Some(value / 100.0)
    } else {
        Some(value)
    }
}

/// `(Target Price - Price) / Price`, skipping a zero price.
fn target_price_upside(
    row: &CanonicalRow,
    target_price: Option<f64>,
    price: Option<f64>,
) -> Option<f64> {
    if !row.contains_key("Target Price") || !row.contains_key("Price") {
        return None;
    }
    let target_price = target_price?;
    let price = price?;
    if price == 0.0 {
        return None;
    }
    Some((target_price - price) / price)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_row(pairs: &[(&str, &str)]) -> FinvizRow {
        pairs
            .iter()
            .map(|(header, value)| {
                (
                    (*header).to_string(),
                    FinvizCell::Text((*value).to_string()),
                )
            })
            .collect()
    }

    #[test]
    fn renames_rescales_and_rounds_non_score_metrics() {
        let row = text_row(&[
            ("Market Cap", "1234.567"),
            ("Forward P/E", "25.555"),
            ("PEG", "1.234"),
            ("P/S", "5.678"),
            ("P/FCF", "44.444"),
            ("EPS Past 5Y", "12.345%"),
            ("EPS Q/Q", "40%"),
            ("Sales Past 5Y", "6.789%"),
            ("Sales Q/Q", "14%"),
            ("ROE", "10.456%"),
            ("ROIC", "9.876%"),
            ("Profit Margin", "20.129%"),
            ("Gross M", "60.5%"),
            ("Oper M", "12.5%"),
            ("Debt/Equity", "0.876"),
            ("Short Float", "3.5%"),
            ("SMA200", "4.5%"),
            ("52W High", "-8%"),
            ("Target Price", "250.50"),
            ("Price", "200.25"),
        ]);

        let normalized = FundamentalDataNormalizer.normalize(vec![row]);
        let stock = &normalized[0];

        assert_eq!(stock.market_cap, Some(1234.567));
        assert_eq!(stock.forward_pe, Some(25.555));
        assert_eq!(stock.peg, Some(1.234));
        assert_eq!(stock.ps, Some(5.678));
        assert_eq!(stock.pfcf, Some(44.444));
        assert_eq!(stock.eps_past_5y, Some(0.1234));
        assert_eq!(stock.eps_quarter_over_quarter, Some(0.4));
        assert_eq!(stock.sales_past_5y, Some(0.0679));
        assert_eq!(stock.sales_quarter_over_quarter, Some(0.14));
        assert_eq!(stock.roe, Some(0.1046));
        assert_eq!(stock.roic, Some(0.0988));
        assert_eq!(stock.profit_margin, Some(0.2013));
        assert_eq!(stock.gross_margin, Some(0.605));
        assert_eq!(stock.operating_margin, Some(0.125));
        assert_eq!(stock.debt_equity, Some(0.876));
        assert_eq!(stock.short_interest, Some(0.035));
        assert_eq!(stock.sma200, Some(0.045));
        assert_eq!(stock.high_52w, Some(-0.08));
        assert_eq!(stock.target_price, Some(250.5));
        assert_eq!(stock.target_price_upside, Some(0.2509));
    }

    #[test]
    fn reads_the_daily_move_from_the_renamed_change_column() {
        let row: FinvizRow = [("Change %".to_string(), FinvizCell::Number(Some(-0.0117)))]
            .into_iter()
            .collect();

        let normalized = FundamentalDataNormalizer.normalize(vec![row]);

        assert_eq!(normalized[0].change, Some(-0.0117));
    }

    #[test]
    fn keeps_already_scaled_numeric_percent_columns() {
        let row: FinvizRow = [("ROE".to_string(), FinvizCell::Number(Some(0.125)))]
            .into_iter()
            .collect();

        let normalized = FundamentalDataNormalizer.normalize(vec![row]);

        assert_eq!(normalized[0].roe, Some(0.125));
    }

    #[test]
    fn removes_blank_tickers_and_leaves_other_rows_alone() {
        let rows = vec![
            text_row(&[("Ticker", "AAPL")]),
            text_row(&[("Ticker", " ")]),
        ];

        let cleaned = FundamentalDataNormalizer.remove_invalid_rows(rows);

        assert_eq!(cleaned.len(), 1);
        assert_eq!(cleaned[0].get("Ticker").unwrap().text(), Some("AAPL"));
    }

    #[test]
    fn keeps_rows_when_the_ticker_column_is_missing() {
        let rows = vec![text_row(&[("Company", "Apple")])];

        assert_eq!(FundamentalDataNormalizer.remove_invalid_rows(rows).len(), 1);
    }

    #[test]
    fn does_not_derive_upside_without_both_inputs() {
        let rows = vec![text_row(&[("Price", "110")])];

        let normalized = FundamentalDataNormalizer.normalize(rows);

        assert_eq!(normalized[0].target_price_upside, None);
    }
}
