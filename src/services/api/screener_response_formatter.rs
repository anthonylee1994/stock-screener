//! Shapes screener rows into the JSON the API returns.

use serde_json::{Map, Value, json};

use crate::models::StockRow;

pub fn format_response(
    rows: &[StockRow],
    total_count: Option<i64>,
    limit: Option<i64>,
    offset: i64,
    has_more: bool,
) -> Value {
    let next_offset = if has_more {
        Value::from(offset + rows.len() as i64)
    } else {
        Value::Null
    };

    json!({
        "data": rows.iter().map(format_record).collect::<Vec<Value>>(),
        "count": total_count.unwrap_or(rows.len() as i64),
        "limit": limit.unwrap_or(rows.len() as i64),
        "offset": offset,
        "has_more": has_more,
        "next_offset": next_offset,
    })
}

pub fn format_record(row: &StockRow) -> Value {
    json!({
        "ticker": optional_text(row.ticker.as_deref()),
        "name": optional_text(row.company.as_deref()),
        "sector": optional_text(row.sector.as_deref()),
        "market_cap": optional_number(row.market_cap),
        "price": optional_number(row.price.or(row.quote_price)),
        "target_price_upside": optional_number(row.target_price_upside),
        "change": optional_number(calculate_change(row)),
        "change_percent": optional_number(row.change.or(row.quote_change_percent)),
        "volume": optional_number(row.volume),
        "total_score": optional_number(row.total_score),
        "potential_stock": match row.potential_stock {
            Some(value) => Value::Bool(value),
            None => Value::Null,
        },
        "fundamental": format_fundamental(row),
        "technical": format_technical(row),
    })
}

fn format_fundamental(row: &StockRow) -> Value {
    let mut fields = Map::new();
    fields.insert("market_cap".into(), optional_number(row.market_cap));
    fields.insert("forward_pe".into(), optional_number(row.forward_pe));
    fields.insert("peg".into(), optional_number(row.peg));
    fields.insert("ps".into(), optional_number(row.ps));
    fields.insert("pfcf".into(), optional_number(row.pfcf));
    fields.insert("eps_past_5y".into(), optional_number(row.eps_past_5y));
    fields.insert("sales_past_5y".into(), optional_number(row.sales_past_5y));
    fields.insert("roe".into(), optional_number(row.roe));
    fields.insert("roic".into(), optional_number(row.roic));
    fields.insert("profit_margin".into(), optional_number(row.profit_margin));
    fields.insert("gross_margin".into(), optional_number(row.gross_margin));
    fields.insert("debt_equity".into(), optional_number(row.debt_equity));
    fields.insert(
        "eps_quarter_over_quarter".into(),
        optional_number(row.eps_quarter_over_quarter),
    );
    fields.insert(
        "sales_quarter_over_quarter".into(),
        optional_number(row.sales_quarter_over_quarter),
    );
    fields.insert(
        "operating_margin".into(),
        optional_number(row.operating_margin),
    );
    fields.insert("short_interest".into(), optional_number(row.short_interest));
    fields.insert("sma200".into(), optional_number(row.sma200));
    fields.insert("high_52w".into(), optional_number(row.high_52w));
    fields.insert("target_price".into(), optional_number(row.target_price));
    fields.insert(
        "target_price_upside".into(),
        optional_number(row.target_price_upside),
    );
    // The raw stored column, so this stays 0/1 rather than the top-level bool.
    fields.insert(
        "potential_stock".into(),
        match row.potential_stock {
            Some(value) => Value::from(i64::from(value)),
            None => Value::Null,
        },
    );
    fields.insert(
        "market_cap_score".into(),
        optional_number(row.market_cap_score),
    );
    fields.insert(
        "forward_pe_score".into(),
        optional_number(row.forward_pe_score),
    );
    fields.insert("peg_score".into(), optional_number(row.peg_score));
    fields.insert("ps_score".into(), optional_number(row.ps_score));
    fields.insert("pfcf_score".into(), optional_number(row.pfcf_score));
    fields.insert(
        "eps_past_5y_score".into(),
        optional_number(row.eps_past_5y_score),
    );
    fields.insert(
        "sales_past_5y_score".into(),
        optional_number(row.sales_past_5y_score),
    );
    fields.insert("roe_score".into(), optional_number(row.roe_score));
    fields.insert("roic_score".into(), optional_number(row.roic_score));
    fields.insert(
        "profit_margin_score".into(),
        optional_number(row.profit_margin_score),
    );
    fields.insert(
        "gross_margin_score".into(),
        optional_number(row.gross_margin_score),
    );
    fields.insert(
        "debt_equity_score".into(),
        optional_number(row.debt_equity_score),
    );
    fields.insert(
        "fundamental_score".into(),
        optional_number(row.fundamental_score),
    );
    Value::Object(fields)
}

fn format_technical(row: &StockRow) -> Value {
    json!({
        "long_term_score": optional_number(row.long_term_score),
        "mid_term_score": optional_number(row.mid_term_score),
        "short_term_score": optional_number(row.short_term_score),
        "technical_score": optional_number(row.technical_score),
    })
}

/// Back out the absolute move from price and the change ratio, falling back to
/// the quote's own change.
fn calculate_change(row: &StockRow) -> Option<f64> {
    if let (Some(price), Some(change_ratio)) = (row.price, row.change)
        && change_ratio != -1.0
    {
        let previous_price = price / (1.0 + change_ratio);
        return Some(price - previous_price);
    }
    row.quote_change
}

fn optional_number(value: Option<f64>) -> Value {
    match value {
        Some(value) => Value::from(value),
        None => Value::Null,
    }
}

fn optional_text(value: Option<&str>) -> Value {
    match value {
        Some(value) => Value::from(value),
        None => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_records_and_pagination_metadata() {
        let rows = vec![StockRow {
            company: Some("NVIDIA Corp".to_string()),
            sector: Some("Technology".to_string()),
            market_cap: Some(5_000_000_000_000.0),
            price: Some(110.0),
            change: Some(0.1),
            volume: Some(2_000_000.0),
            total_score: Some(88.8),
            potential_stock: Some(true),
            gross_margin: Some(0.65),
            sma200: Some(0.02),
            target_price: Some(150.0),
            target_price_upside: Some(0.3636),
            fundamental_score: Some(90.0),
            technical_score: Some(87.0),
            ..StockRow::with_ticker("NVDA")
        }];

        let response = format_response(&rows, Some(3), Some(1), 1, true);

        assert_eq!(response["count"], 3);
        assert_eq!(response["limit"], 1);
        assert_eq!(response["offset"], 1);
        assert_eq!(response["has_more"], true);
        assert_eq!(response["next_offset"], 2);
        let record = &response["data"][0];
        assert_eq!(record["ticker"], "NVDA");
        assert!((record["change"].as_f64().unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(record["target_price_upside"], 0.3636);
        assert_eq!(record["potential_stock"], true);
        assert_eq!(record["fundamental"]["potential_stock"], 1);
        assert_eq!(record["fundamental"]["target_price_upside"], 0.3636);
        assert_eq!(record["fundamental"]["gross_margin"], 0.65);
        assert_eq!(record["fundamental"]["sma200"], 0.02);
        assert_eq!(record["fundamental"]["fundamental_score"], 90.0);
        assert_eq!(record["technical"]["technical_score"], 87.0);
    }

    #[test]
    fn prefers_screener_fields_over_quote_fields() {
        let row = StockRow {
            quote_price: Some(105.0),
            quote_change: Some(4.0),
            quote_change_percent: Some(2.5),
            price: Some(110.0),
            change: Some(0.1),
            ..StockRow::with_ticker("MSFT")
        };

        let record = format_record(&row);

        assert_eq!(record["price"], 110.0);
        assert!((record["change"].as_f64().unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(record["change_percent"], 0.1);
    }

    #[test]
    fn falls_back_to_the_quote_change_for_a_total_wipeout() {
        let row = StockRow {
            price: Some(0.0),
            change: Some(-1.0),
            quote_change: Some(-4.0),
            ..StockRow::with_ticker("MSFT")
        };

        assert_eq!(format_record(&row)["change"], -4.0);
    }

    #[test]
    fn uses_the_quote_change_when_the_change_percent_is_missing() {
        let row = StockRow {
            quote_change: Some(4.0),
            ..StockRow::with_ticker("MSFT")
        };

        assert_eq!(format_record(&row)["change"], 4.0);
    }

    #[test]
    fn never_derives_a_missing_target_price_upside() {
        let row = StockRow {
            price: Some(110.0),
            target_price: Some(125.5),
            ..StockRow::with_ticker("MSFT")
        };

        assert_eq!(format_record(&row)["target_price_upside"], Value::Null);
    }

    #[test]
    fn returns_null_for_missing_values() {
        let record = format_record(&StockRow::with_ticker("AAPL"));

        assert_eq!(record["price"], Value::Null);
        assert_eq!(record["change"], Value::Null);
        assert_eq!(record["total_score"], Value::Null);
        assert_eq!(record["potential_stock"], Value::Null);
    }

    #[test]
    fn reports_no_next_offset_on_the_last_page() {
        let response = format_response(&[StockRow::with_ticker("AAPL")], None, None, 0, false);

        assert_eq!(response["count"], 1);
        assert_eq!(response["limit"], 1);
        assert_eq!(response["next_offset"], Value::Null);
    }
}
