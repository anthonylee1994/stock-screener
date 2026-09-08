//! Momentum and trend indicators derived from daily closes.

use crate::models::TechnicalRow;
use crate::services::technical::technical_price_client::PriceData;

/// A ticker needs at least this many closes before any indicator is trusted.
const MIN_CLOSE_COUNT: usize = 201;

pub fn calculate_indicators(price_data: &PriceData, tickers: &[String]) -> Vec<TechnicalRow> {
    tickers
        .iter()
        .filter_map(|ticker| calculate_ticker_indicators(ticker, price_data))
        .collect()
}

pub fn calculate_ticker_indicators(ticker: &str, price_data: &PriceData) -> Option<TechnicalRow> {
    let series = price_data.get(ticker)?;
    let close: Vec<f64> = series.close.iter().flatten().copied().collect();
    if close.len() < MIN_CLOSE_COUNT {
        return None;
    }

    let latest_close = close[close.len() - 1];
    let previous_close = close[close.len() - 2];
    let ema200 = *ewm(&close, span_alpha(200.0)).last()?;
    let ema50 = *ewm(&close, span_alpha(50.0)).last()?;
    let ppo = calculate_ppo(&close);
    let volume = series.volume.iter().flatten().last().copied();

    Some(TechnicalRow {
        ticker: ticker.to_string(),
        quote_price: finite(latest_close),
        quote_change: finite(latest_close - previous_close),
        quote_change_percent: finite((latest_close - previous_close) / previous_close),
        quote_volume: volume,
        ema200_distance: finite((latest_close / ema200) - 1.0),
        roc125: finite((latest_close / close[close.len() - 126]) - 1.0),
        ema50_distance: finite((latest_close / ema50) - 1.0),
        roc20: finite((latest_close / close[close.len() - 21]) - 1.0),
        ppo_slope3: finite((ppo[ppo.len() - 1] - ppo[ppo.len() - 4]) / 3.0),
        rsi14: calculate_rsi14(&close),
        ..TechnicalRow::default()
    })
}

/// Percentage price oscillator, in percent.
fn calculate_ppo(close: &[f64]) -> Vec<f64> {
    let ema12 = ewm(close, span_alpha(12.0));
    let ema26 = ewm(close, span_alpha(26.0));
    ema12
        .iter()
        .zip(ema26.iter())
        .map(|(fast, slow)| ((fast - slow) / slow) * 100.0)
        .collect()
}

/// Wilder-smoothed RSI over 14 periods.
fn calculate_rsi14(close: &[f64]) -> Option<f64> {
    let deltas: Vec<f64> = close.windows(2).map(|pair| pair[1] - pair[0]).collect();
    if deltas.is_empty() {
        return None;
    }
    let gains: Vec<f64> = deltas.iter().map(|delta| delta.max(0.0)).collect();
    let losses: Vec<f64> = deltas.iter().map(|delta| (-delta).max(0.0)).collect();

    let alpha = 1.0 / 14.0;
    let average_gain = *ewm(&gains, alpha).last()?;
    let average_loss = *ewm(&losses, alpha).last()?;
    let relative_strength = average_gain / average_loss;
    finite(100.0 - (100.0 / (1.0 + relative_strength)))
}

/// `pandas.Series.ewm(span=..., adjust=False)`.
fn span_alpha(span: f64) -> f64 {
    2.0 / (span + 1.0)
}

fn ewm(values: &[f64], alpha: f64) -> Vec<f64> {
    let mut smoothed = Vec::with_capacity(values.len());
    let mut current = 0.0;
    for (index, value) in values.iter().enumerate() {
        current = if index == 0 {
            *value
        } else {
            alpha * value + (1.0 - alpha) * current
        };
        smoothed.push(current);
    }
    smoothed
}

/// Division by zero produces infinities the scoring stage cannot rank, so any
/// non-finite result becomes a missing value.
fn finite(value: f64) -> Option<f64> {
    if value.is_nan() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::technical::technical_price_client::PriceSeries;

    fn rising_price_data() -> PriceData {
        let close = (1..=202).map(|value| Some(value as f64)).collect();
        let volume = (1000..1202).map(|value| Some(value as f64)).collect();
        [("AAPL".to_string(), PriceSeries { close, volume })]
            .into_iter()
            .collect()
    }

    #[test]
    fn calculates_rows_for_valid_tickers() {
        let price_data = rising_price_data();

        let rows = calculate_indicators(&price_data, &["AAPL".to_string()]);
        let row = &rows[0];

        assert_eq!(row.ticker, "AAPL");
        assert_eq!(row.quote_price, Some(202.0));
        assert_eq!(row.quote_change, Some(1.0));
        assert!((row.quote_change_percent.unwrap() - 1.0 / 201.0).abs() < 1e-12);
        assert_eq!(row.quote_volume, Some(1201.0));
        assert!(row.ema200_distance.unwrap() > 0.0);
        assert!((row.roc125.unwrap() - ((202.0 / 77.0) - 1.0)).abs() < 1e-12);
        assert!((row.roc20.unwrap() - ((202.0 / 182.0) - 1.0)).abs() < 1e-12);
        assert!(row.ppo_slope3.is_some());
        assert_eq!(row.rsi14, Some(100.0));
    }

    #[test]
    fn skips_missing_or_short_history() {
        let price_data: PriceData = [(
            "AAPL".to_string(),
            PriceSeries {
                close: vec![Some(1.0), Some(2.0)],
                volume: vec![Some(1.0), Some(2.0)],
            },
        )]
        .into_iter()
        .collect();

        assert!(calculate_ticker_indicators("MSFT", &price_data).is_none());
        assert!(calculate_ticker_indicators("AAPL", &price_data).is_none());
    }

    #[test]
    fn reports_no_volume_when_every_bar_is_missing() {
        let mut price_data = rising_price_data();
        price_data.get_mut("AAPL").unwrap().volume = vec![None; 202];

        let row = calculate_ticker_indicators("AAPL", &price_data).unwrap();

        assert_eq!(row.quote_volume, None);
    }

    #[test]
    fn smooths_exponentially_without_adjustment() {
        let smoothed = ewm(&[1.0, 2.0, 3.0], 0.5);

        assert_eq!(smoothed, vec![1.0, 1.5, 2.25]);
    }
}
