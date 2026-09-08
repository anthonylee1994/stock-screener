//! Ticker extraction from a Finviz screener table cell.

use scraper::{ElementRef, Selector};

/// Extract the real ticker from a Finviz screener table cell.
///
/// Finviz renders a logo fallback letter plus the ticker link, so the plain
/// cell text becomes values like `NNVDA` instead of `NVDA`. Prefer the explicit
/// ticker attribute or the tab-link text when present.
pub fn extract_ticker_from_cell(cell: ElementRef) -> String {
    if let Some(attribute_ticker) = cell.value().attr("data-boxover-ticker") {
        let attribute_ticker = attribute_ticker.trim();
        if !attribute_ticker.is_empty() {
            return attribute_ticker.to_string();
        }
    }

    let tab_link_selector = Selector::parse("a.tab-link").expect("valid selector");
    if let Some(tab_link) = cell.select(&tab_link_selector).next() {
        let link_text = collapse_text(&tab_link);
        if !link_text.is_empty() {
            return link_text;
        }
    }

    collapse_text(&cell)
}

/// BeautifulSoup's `get_text(strip=True)`: strip every text node, then join.
fn collapse_text(element: &ElementRef) -> String {
    element
        .text()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;

    /// `<td>` only survives parsing inside a table, so wrap it in one.
    fn first_cell(html: &str) -> (Html, Selector) {
        (
            Html::parse_document(&format!("<table><tr>{html}</tr></table>")),
            Selector::parse("td").unwrap(),
        )
    }

    #[test]
    fn prefers_data_boxover_ticker() {
        let html = r#"
        <td align="left" data-boxover-company="NVIDIA Corp" data-boxover-ticker="NVDA" height="10">
          <span class="flex items-center gap-1 pl-0.5">
            <a class="company-ticker" href="stock?t=NVDA">
              <img alt="NVDA logo" src="https://logo.finviz.com/NVDA.svg"/>
              <span>N</span>
            </a>
            <a class="tab-link" href="stock?t=NVDA">NVDA</a>
          </span>
        </td>
        "#;
        let (document, selector) = first_cell(html);
        let cell = document.select(&selector).next().unwrap();

        assert_eq!(collapse_text(&cell), "NNVDA");
        assert_eq!(extract_ticker_from_cell(cell), "NVDA");
    }

    #[test]
    fn falls_back_to_tab_link() {
        let html =
            r#"<td align="left" height="10"><a class="tab-link" href="stock?t=AAPL">AAPL</a></td>"#;
        let (document, selector) = first_cell(html);
        let cell = document.select(&selector).next().unwrap();

        assert_eq!(extract_ticker_from_cell(cell), "AAPL");
    }

    #[test]
    fn falls_back_to_plain_text() {
        let (document, selector) = first_cell("<td>MSFT</td>");
        let cell = document.select(&selector).next().unwrap();

        assert_eq!(extract_ticker_from_cell(cell), "MSFT");
    }
}
