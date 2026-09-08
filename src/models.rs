//! Row types shared by every screener stage.
//!
//! The Python version passed `pandas.DataFrame` objects around. The schema is
//! fixed, so here every stage works on typed rows instead: [`StockRow`] carries
//! the full persisted schema and [`TechnicalRow`] carries the technical stage
//! output before it is merged back into [`StockRow`].

/// A single value as it is written to, or read from, SQLite.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Null,
    Text(String),
    Real(f64),
    Integer(i64),
}

impl SqlValue {
    pub fn text(value: Option<&str>) -> Self {
        match value {
            Some(value) => SqlValue::Text(value.to_string()),
            None => SqlValue::Null,
        }
    }

    pub fn real(value: Option<f64>) -> Self {
        match value {
            Some(value) if value.is_finite() => SqlValue::Real(value),
            _ => SqlValue::Null,
        }
    }

    pub fn boolean(value: Option<bool>) -> Self {
        match value {
            Some(value) => SqlValue::Integer(i64::from(value)),
            None => SqlValue::Null,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StockRow {
    pub ticker: Option<String>,
    pub company: Option<String>,
    pub sector: Option<String>,
    pub market_cap: Option<f64>,
    pub market_cap_score: Option<f64>,
    pub forward_pe: Option<f64>,
    pub forward_pe_score: Option<f64>,
    pub peg: Option<f64>,
    pub peg_score: Option<f64>,
    pub ps: Option<f64>,
    pub ps_score: Option<f64>,
    pub pfcf: Option<f64>,
    pub pfcf_score: Option<f64>,
    pub eps_past_5y: Option<f64>,
    pub eps_past_5y_score: Option<f64>,
    pub sales_past_5y: Option<f64>,
    pub sales_past_5y_score: Option<f64>,
    pub eps_quarter_over_quarter: Option<f64>,
    pub sales_quarter_over_quarter: Option<f64>,
    pub roe: Option<f64>,
    pub roe_score: Option<f64>,
    pub roic: Option<f64>,
    pub roic_score: Option<f64>,
    pub profit_margin: Option<f64>,
    pub profit_margin_score: Option<f64>,
    pub gross_margin: Option<f64>,
    pub gross_margin_score: Option<f64>,
    pub operating_margin: Option<f64>,
    pub debt_equity: Option<f64>,
    pub debt_equity_score: Option<f64>,
    pub short_interest: Option<f64>,
    pub sma200: Option<f64>,
    pub high_52w: Option<f64>,
    pub target_price: Option<f64>,
    pub target_price_upside: Option<f64>,
    pub potential_stock: Option<bool>,
    pub price: Option<f64>,
    pub change: Option<f64>,
    pub volume: Option<f64>,
    pub quote_price: Option<f64>,
    pub quote_change: Option<f64>,
    pub quote_change_percent: Option<f64>,
    pub quote_volume: Option<f64>,
    pub fundamental_score: Option<f64>,
    pub long_term_score: Option<f64>,
    pub mid_term_score: Option<f64>,
    pub short_term_score: Option<f64>,
    pub technical_score: Option<f64>,
    pub ema200_distance: Option<f64>,
    pub roc125: Option<f64>,
    pub ema50_distance: Option<f64>,
    pub roc20: Option<f64>,
    pub ppo_slope3: Option<f64>,
    pub rsi14: Option<f64>,
    pub total_score: Option<f64>,
}

impl StockRow {
    pub fn with_ticker(ticker: &str) -> Self {
        StockRow {
            ticker: Some(ticker.to_string()),
            ..StockRow::default()
        }
    }

    pub fn ticker_str(&self) -> &str {
        self.ticker.as_deref().unwrap_or("")
    }

    /// Values in the exact order of [`crate::utils::stock_schema::STOCKS_COLUMNS`].
    pub fn sql_values(&self) -> Vec<SqlValue> {
        vec![
            SqlValue::text(self.ticker.as_deref()),
            SqlValue::text(self.company.as_deref()),
            SqlValue::text(self.sector.as_deref()),
            SqlValue::real(self.market_cap),
            SqlValue::real(self.market_cap_score),
            SqlValue::real(self.forward_pe),
            SqlValue::real(self.forward_pe_score),
            SqlValue::real(self.peg),
            SqlValue::real(self.peg_score),
            SqlValue::real(self.ps),
            SqlValue::real(self.ps_score),
            SqlValue::real(self.pfcf),
            SqlValue::real(self.pfcf_score),
            SqlValue::real(self.eps_past_5y),
            SqlValue::real(self.eps_past_5y_score),
            SqlValue::real(self.sales_past_5y),
            SqlValue::real(self.sales_past_5y_score),
            SqlValue::real(self.eps_quarter_over_quarter),
            SqlValue::real(self.sales_quarter_over_quarter),
            SqlValue::real(self.roe),
            SqlValue::real(self.roe_score),
            SqlValue::real(self.roic),
            SqlValue::real(self.roic_score),
            SqlValue::real(self.profit_margin),
            SqlValue::real(self.profit_margin_score),
            SqlValue::real(self.gross_margin),
            SqlValue::real(self.gross_margin_score),
            SqlValue::real(self.operating_margin),
            SqlValue::real(self.debt_equity),
            SqlValue::real(self.debt_equity_score),
            SqlValue::real(self.short_interest),
            SqlValue::real(self.sma200),
            SqlValue::real(self.high_52w),
            SqlValue::real(self.target_price),
            SqlValue::real(self.target_price_upside),
            // A row that never went through the potential-stock filter is
            // stored as `false`, so the `= 1` screener filter stays simple.
            SqlValue::boolean(Some(self.potential_stock.unwrap_or(false))),
            SqlValue::real(self.price),
            SqlValue::real(self.change),
            SqlValue::real(self.volume),
            SqlValue::real(self.quote_price),
            SqlValue::real(self.quote_change),
            SqlValue::real(self.quote_change_percent),
            SqlValue::real(self.quote_volume),
            SqlValue::real(self.fundamental_score),
            SqlValue::real(self.long_term_score),
            SqlValue::real(self.mid_term_score),
            SqlValue::real(self.short_term_score),
            SqlValue::real(self.technical_score),
            SqlValue::real(self.ema200_distance),
            SqlValue::real(self.roc125),
            SqlValue::real(self.ema50_distance),
            SqlValue::real(self.roc20),
            SqlValue::real(self.ppo_slope3),
            SqlValue::real(self.rsi14),
            SqlValue::real(self.total_score),
        ]
    }

    /// Copy the technical stage output for this ticker onto the row. Mirrors the
    /// Python `fundamental_data.merge(technical_data, on="Ticker", how="left")`.
    pub fn merge_technical(&mut self, technical: &TechnicalRow) {
        self.quote_price = technical.quote_price;
        self.quote_change = technical.quote_change;
        self.quote_change_percent = technical.quote_change_percent;
        self.quote_volume = technical.quote_volume;
        self.long_term_score = technical.long_term_score;
        self.mid_term_score = technical.mid_term_score;
        self.short_term_score = technical.short_term_score;
        self.technical_score = technical.technical_score;
        self.ema200_distance = technical.ema200_distance;
        self.roc125 = technical.roc125;
        self.ema50_distance = technical.ema50_distance;
        self.roc20 = technical.roc20;
        self.ppo_slope3 = technical.ppo_slope3;
        self.rsi14 = technical.rsi14;
    }
}

/// Technical stage row, matching the Python `DEFAULT_COLUMNS` of
/// `technical_score_calculator`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TechnicalRow {
    pub ticker: String,
    pub quote_price: Option<f64>,
    pub quote_change: Option<f64>,
    pub quote_change_percent: Option<f64>,
    pub quote_volume: Option<f64>,
    pub long_term_score: Option<f64>,
    pub mid_term_score: Option<f64>,
    pub short_term_score: Option<f64>,
    pub technical_score: Option<f64>,
    pub ema200_distance: Option<f64>,
    pub roc125: Option<f64>,
    pub ema50_distance: Option<f64>,
    pub roc20: Option<f64>,
    pub ppo_slope3: Option<f64>,
    pub rsi14: Option<f64>,
}
