//! Text-to-number coercion and rounding shared by every scoring stage.

/// Port of the Python `to_numeric_series` applied to a single cell: drop `%`
/// and thousands separators, treat `-`, `nan` and `None` as missing, then parse.
pub fn parse_numeric(text: &str) -> Option<f64> {
    let stripped: String = text.chars().filter(|c| *c != '%' && *c != ',').collect();
    if matches!(stripped.as_str(), "-" | "nan" | "None") {
        return None;
    }
    stripped.trim().parse::<f64>().ok().filter(|v| !v.is_nan())
}

/// `numpy.round` semantics: round half to even, so scores match the Python
/// pipeline bit for bit on tie values.
pub fn round_to(value: f64, digits: u32) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let factor = 10f64.powi(digits as i32);
    let scaled = value * factor;
    round_half_even(scaled) / factor
}

pub fn round_option(value: Option<f64>, digits: u32) -> Option<f64> {
    value.map(|value| round_to(value, digits))
}

fn round_half_even(value: f64) -> f64 {
    let fraction = value - value.trunc();
    if fraction.abs() != 0.5 {
        return value.round();
    }
    let floor = value.floor();
    if (floor as i64) % 2 == 0 {
        floor
    } else {
        floor + 1.0
    }
}

/// Clip to an upper bound, keeping missing values missing.
pub fn clip_upper(value: Option<f64>, upper: f64) -> Option<f64> {
    value.map(|value| value.min(upper))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_percent_and_thousand_separators() {
        assert_eq!(parse_numeric("12.5%"), Some(12.5));
        assert_eq!(parse_numeric("1,234.5"), Some(1234.5));
        assert_eq!(parse_numeric("-"), None);
        assert_eq!(parse_numeric("nan"), None);
        assert_eq!(parse_numeric("None"), None);
        assert_eq!(parse_numeric(""), None);
        assert_eq!(parse_numeric("-3.5"), Some(-3.5));
    }

    #[test]
    fn rounds_half_to_even_like_numpy() {
        assert_eq!(round_to(2.5, 0), 2.0);
        assert_eq!(round_to(3.5, 0), 4.0);
        assert_eq!(round_to(-2.5, 0), -2.0);
        assert_eq!(round_to(83.333_333, 2), 83.33);
        assert_eq!(round_to(0.123_45, 4), 0.1234);
    }
}
