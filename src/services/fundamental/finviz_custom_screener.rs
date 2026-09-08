//! Finviz screener HTML parsing, replacing the `finvizfinance` Custom screener.

use std::collections::HashMap;

use scraper::{ElementRef, Html, Selector};

use crate::services::common::series_normalizer::parse_numeric;
use crate::services::fundamental::finviz_ticker::extract_ticker_from_cell;

/// Finviz headers whose cells hold numbers, taken from `finvizfinance`'s
/// `NUMBER_COL`.
pub const NUMBER_COL: &[&str] = &[
    "Market Cap",
    "P/E",
    "Fwd P/E",
    "PEG",
    "P/S",
    "P/B",
    "P/C",
    "P/FCF",
    "Dividend",
    "Payout Ratio",
    "EPS",
    "EPS this Y",
    "EPS next Y",
    "EPS past 5Y",
    "EPS next 5Y",
    "Sales past 5Y",
    "EPS Q/Q",
    "Sales Q/Q",
    "Outstanding",
    "Float",
    "Insider Own",
    "Insider Trans",
    "Inst Own",
    "Inst Trans",
    "Float Short",
    "Short Ratio",
    "ROA",
    "ROE",
    "ROI",
    "Curr R",
    "Quick R",
    "LTDebt/Eq",
    "Debt/Eq",
    "Gross M",
    "Oper M",
    "Profit M",
    "Perf Week",
    "Perf Month",
    "Perf Quart",
    "Perf Half",
    "Perf Year",
    "Perf YTD",
    "Beta",
    "ATR",
    "Volatility W",
    "Volatility M",
    "SMA20",
    "SMA50",
    "SMA200",
    "50D High",
    "50D Low",
    "52W High",
    "52W Low",
    "RSI",
    "from Open",
    "Gap",
    "Recom",
    "Avg Volume",
    "Rel Volume",
    "Price",
    "Change",
    "Volume",
    "Target Price",
];

/// One parsed screener cell. Numeric headers keep the converted number, every
/// other header keeps the raw cell text so the normalizer can still spot a `%`.
#[derive(Debug, Clone, PartialEq)]
pub enum FinvizCell {
    Text(String),
    Number(Option<f64>),
}

impl FinvizCell {
    pub fn numeric(&self) -> Option<f64> {
        match self {
            FinvizCell::Text(text) => parse_numeric(text),
            FinvizCell::Number(number) => *number,
        }
    }

    pub fn has_percent_symbol(&self) -> bool {
        match self {
            FinvizCell::Text(text) => text.contains('%'),
            FinvizCell::Number(_) => false,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            FinvizCell::Text(text) => Some(text),
            FinvizCell::Number(_) => None,
        }
    }
}

/// A screener row keyed by its Finviz header.
pub type FinvizRow = HashMap<String, FinvizCell>;

/// Convert a Finviz number string to a float, matching `finvizfinance`'s
/// `number_covert`: `%` divides by 100, `B`/`M`/`K` scale up.
pub fn number_covert(text: &str) -> Option<f64> {
    if text.is_empty() || text == "-" {
        return None;
    }
    let text = text.trim();
    let (body, factor) = match text.chars().last()? {
        '%' => (&text[..text.len() - 1], 0.01),
        'B' => (&text[..text.len() - 1], 1_000_000_000.0),
        'M' => (&text[..text.len() - 1], 1_000_000.0),
        'K' => (&text[..text.len() - 1], 1_000.0),
        _ => (text, 1.0),
    };
    let body = body.replace(',', "");
    body.parse::<f64>().ok().map(|value| value * factor)
}

/// A single screener page: its header row, its data rows and the number of
/// pages the screener reports.
pub struct ScreenerPage {
    pub headers: Vec<String>,
    pub rows: Vec<FinvizRow>,
    pub page_count: usize,
}

pub fn parse_screener_page(
    html: &str,
    known_headers: Option<&[String]>,
    limit: i64,
) -> Option<ScreenerPage> {
    let document = Html::parse_document(html);
    let page_count = parse_page_count(&document);
    if page_count == 0 {
        return None;
    }

    let table_selector = Selector::parse("table.screener_table").expect("valid selector");
    let row_selector = Selector::parse("tr").expect("valid selector");
    let table = document.select(&table_selector).next()?;
    let table_rows: Vec<ElementRef> = table.select(&row_selector).collect();

    let headers = match known_headers {
        Some(headers) => headers.to_vec(),
        None => parse_table_header(table_rows.first().copied()?),
    };
    let rows = parse_table_rows(&table_rows, &headers, limit);

    Some(ScreenerPage {
        headers,
        rows,
        page_count,
    })
}

fn parse_page_count(document: &Html) -> usize {
    let option_selector = Selector::parse("#pageSelect option").expect("valid selector");
    document.select(&option_selector).count()
}

fn parse_table_header(header_row: ElementRef) -> Vec<String> {
    let cell_selector = Selector::parse("th").expect("valid selector");
    header_row
        .select(&cell_selector)
        .skip(1)
        .map(|cell| cell.text().collect::<String>().trim().to_string())
        .collect()
}

/// Port of the patched `_get_table`: skip the header row, skip the leading
/// row-number cell, and read the ticker from its attribute instead of the text.
fn parse_table_rows(table_rows: &[ElementRef], headers: &[String], limit: i64) -> Vec<FinvizRow> {
    let cell_selector = Selector::parse("td").expect("valid selector");
    let data_rows = table_rows.iter().skip(1);
    let data_rows: Vec<&ElementRef> = if limit == -1 {
        data_rows.collect()
    } else {
        data_rows.take(limit.max(0) as usize).collect()
    };

    let mut rows = Vec::with_capacity(data_rows.len());
    for row in data_rows {
        let cells: Vec<ElementRef> = row.select(&cell_selector).skip(1).collect();
        let mut parsed_row = FinvizRow::new();
        for (index, cell) in cells.iter().enumerate() {
            let Some(header) = headers.get(index) else {
                continue;
            };
            let value = if header == "Ticker" {
                FinvizCell::Text(extract_ticker_from_cell(*cell))
            } else if NUMBER_COL.contains(&header.as_str()) {
                FinvizCell::Number(number_covert(cell.text().collect::<String>().trim()))
            } else {
                FinvizCell::Text(cell.text().collect::<String>())
            };
            parsed_row.insert(header.clone(), value);
        }
        rows.push(parsed_row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOUBLED_TICKER_CELL: &str = r#"
    <td align="left" data-boxover-company="NVIDIA Corp" data-boxover-ticker="NVDA" height="10">
      <span><a class="company-ticker" href="stock?t=NVDA"><span>N</span></a>
      <a class="tab-link" href="stock?t=NVDA">NVDA</a></span>
    </td>
    "#;

    fn page_html(body: &str) -> String {
        format!(
            r#"<html><body>
            <select id="pageSelect"><option>1</option></select>
            <table class="screener_table">
              <tr><th>No.</th><th>Ticker</th><th>Company</th><th>Market Cap</th></tr>
              {body}
            </table>
            </body></html>"#
        )
    }

    #[test]
    fn converts_finviz_number_suffixes() {
        assert_eq!(number_covert("12.5%"), Some(0.125));
        assert_eq!(number_covert("3.2B"), Some(3_200_000_000.0));
        assert_eq!(number_covert("1.5M"), Some(1_500_000.0));
        assert_eq!(number_covert("900K"), Some(900_000.0));
        assert_eq!(number_covert("1,234.5"), Some(1234.5));
        assert_eq!(number_covert("-"), None);
        assert_eq!(number_covert(""), None);
    }

    #[test]
    fn parses_ticker_without_the_logo_letter() {
        let html = page_html(&format!(
            "<tr><td>1</td>{DOUBLED_TICKER_CELL}<td>NVIDIA Corp</td><td>3.2B</td></tr>"
        ));

        let page = parse_screener_page(&html, None, -1).expect("page");

        assert_eq!(page.page_count, 1);
        assert_eq!(page.headers, ["Ticker", "Company", "Market Cap"]);
        assert_eq!(page.rows.len(), 1);
        assert_eq!(page.rows[0].get("Ticker").unwrap().text(), Some("NVDA"));
        assert_eq!(
            page.rows[0].get("Market Cap").unwrap().numeric(),
            Some(3_200_000_000.0)
        );
    }

    #[test]
    fn truncates_rows_to_the_limit() {
        let row = format!("<tr><td>1</td>{DOUBLED_TICKER_CELL}<td>NVIDIA</td><td>3.2B</td></tr>");
        let html = page_html(&format!("{row}{row}{row}"));

        let page = parse_screener_page(&html, None, 2).expect("page");

        assert_eq!(page.rows.len(), 2);
    }

    #[test]
    fn returns_nothing_when_the_screener_reports_no_pages() {
        let html = r#"<html><body><table class="screener_table"></table></body></html>"#;

        assert!(parse_screener_page(html, None, -1).is_none());
    }
}
