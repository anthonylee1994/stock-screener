//! Flags stocks that are strong on fundamentals *and* confirmed by momentum.

use crate::models::StockRow;
use crate::services::common::percentile_scorer::percentile_score;

// 基本面硬篩（護城河 + 增長）
const MIN_POTENTIAL_MARKET_CAP: f64 = 10_000_000_000.0; // $10B — large cap universe
const MIN_POTENTIAL_VOLUME: f64 = 1_000_000.0; // Finviz Avg Volume > 1M proxy
const MIN_POTENTIAL_FORWARD_PE: f64 = 1.0; // 正盈利，避免 loss-making 當 cheap
const MAX_POTENTIAL_FORWARD_PE: f64 = 60.0; // 避免估值太離地
const MIN_POTENTIAL_PEG: f64 = 0.01; // 正增長估值
const MAX_POTENTIAL_PEG: f64 = 1.5; // growth-adjusted valuation sanity check
const MIN_POTENTIAL_PFCF: f64 = 1.0; // 正 FCF valuation
const MAX_POTENTIAL_PFCF: f64 = 180.0; // 保留高質 compounder，但排除極端值
const MIN_POTENTIAL_ROE: f64 = 0.15; // 15% — 資本回報 / 護城河
const MIN_POTENTIAL_ROIC: f64 = 0.10; // 10% — capital efficiency
const MIN_POTENTIAL_PROFIT_MARGIN: f64 = 0.10; // 10% — 盈利質素
const MIN_POTENTIAL_EPS_PAST_5Y: f64 = 0.10; // 10% — EPS 成長
const MIN_POTENTIAL_SALES_PAST_5Y: f64 = 0.10; // 10% — Sales 成長
const MAX_POTENTIAL_DEBT_EQUITY: f64 = 1.5; // 資產負債表穩健

// 技術確認（動量 + 趨勢）
const MIN_POTENTIAL_ROC125: f64 = 0.10; // Performance Half Year up 10% proxy
const MAX_POTENTIAL_ROC125: f64 = 4.00; // 避免 split / spin-off artifact
const MIN_POTENTIAL_ROC125_PERCENTILE: f64 = 60.0; // 全市場 60th percentile
const MIN_POTENTIAL_ROC20: f64 = -0.15; // Performance Month not worse than -15%
const MIN_POTENTIAL_RSI: f64 = 35.0;
const MAX_POTENTIAL_RSI: f64 = 75.0; // 唔超買唔超賣

/// Returns one flag per row, in the same order.
///
/// A missing input fails its rule, so every rule needs a value to pass.
pub fn apply(rows: &[StockRow]) -> Vec<bool> {
    let roc125: Vec<Option<f64>> = rows.iter().map(|row| row.roc125).collect();
    let roc125_percentile = percentile_score(&roc125, true);

    rows.iter()
        .enumerate()
        .map(|(index, row)| fundamental_pass(row) && technical_pass(row, roc125_percentile[index]))
        .collect()
}

fn fundamental_pass(row: &StockRow) -> bool {
    at_least(row.market_cap, MIN_POTENTIAL_MARKET_CAP)
        && at_least(row.volume, MIN_POTENTIAL_VOLUME)
        && within(
            row.forward_pe,
            MIN_POTENTIAL_FORWARD_PE,
            MAX_POTENTIAL_FORWARD_PE,
        )
        && within(row.peg, MIN_POTENTIAL_PEG, MAX_POTENTIAL_PEG)
        && within(row.pfcf, MIN_POTENTIAL_PFCF, MAX_POTENTIAL_PFCF)
        && at_least(row.roe, MIN_POTENTIAL_ROE)
        && at_least(row.roic, MIN_POTENTIAL_ROIC)
        && at_least(row.profit_margin, MIN_POTENTIAL_PROFIT_MARGIN)
        && at_least(row.eps_past_5y, MIN_POTENTIAL_EPS_PAST_5Y)
        && at_least(row.sales_past_5y, MIN_POTENTIAL_SALES_PAST_5Y)
        && at_most(row.debt_equity, MAX_POTENTIAL_DEBT_EQUITY)
}

fn technical_pass(row: &StockRow, roc125_percentile: f64) -> bool {
    within(row.roc125, MIN_POTENTIAL_ROC125, MAX_POTENTIAL_ROC125)
        && roc125_percentile >= MIN_POTENTIAL_ROC125_PERCENTILE
        && greater_than(row.roc20, MIN_POTENTIAL_ROC20)
        && greater_than(row.ema200_distance, 0.0)
        && within(row.rsi14, MIN_POTENTIAL_RSI, MAX_POTENTIAL_RSI)
}

fn at_least(value: Option<f64>, minimum: f64) -> bool {
    value.is_some_and(|value| value >= minimum)
}

fn at_most(value: Option<f64>, maximum: f64) -> bool {
    value.is_some_and(|value| value <= maximum)
}

fn greater_than(value: Option<f64>, minimum: f64) -> bool {
    value.is_some_and(|value| value > minimum)
}

fn within(value: Option<f64>, minimum: f64, maximum: f64) -> bool {
    value.is_some_and(|value| value >= minimum && value <= maximum)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一隻條件全部過關嘅 baseline stock。
    fn all_pass_stock(ticker: &str) -> StockRow {
        StockRow {
            market_cap: Some(20_000_000_000.0),
            volume: Some(2_000_000.0),
            forward_pe: Some(25.0),
            peg: Some(1.0),
            pfcf: Some(50.0),
            roe: Some(0.20),
            roic: Some(0.15),
            profit_margin: Some(0.20),
            eps_past_5y: Some(0.20),
            sales_past_5y: Some(0.20),
            debt_equity: Some(1.0),
            roc125: Some(0.20),
            roc20: Some(0.05),
            ema200_distance: Some(0.05),
            rsi14: Some(55.0),
            ..StockRow::with_ticker(ticker)
        }
    }

    /// A single row keeps the ROC125 percentile at 100, so it never interferes
    /// with the field under test.
    fn apply_one(mutate: impl FnOnce(&mut StockRow)) -> bool {
        let mut row = all_pass_stock("T");
        mutate(&mut row);
        apply(&[row])[0]
    }

    #[test]
    fn baseline_passes() {
        assert!(apply_one(|_| {}));
    }

    #[test]
    fn checks_fundamental_boundaries() {
        assert!(apply_one(|row| row.market_cap = Some(10_000_000_000.0)));
        assert!(!apply_one(|row| row.market_cap = Some(9_999_999_999.0)));

        assert!(apply_one(|row| row.volume = Some(1_000_000.0)));
        assert!(!apply_one(|row| row.volume = Some(999_999.0)));

        assert!(apply_one(|row| row.forward_pe = Some(1.0)));
        assert!(apply_one(|row| row.forward_pe = Some(60.0)));
        assert!(!apply_one(|row| row.forward_pe = Some(0.99)));
        assert!(!apply_one(|row| row.forward_pe = Some(60.01)));

        assert!(apply_one(|row| row.peg = Some(0.01)));
        assert!(apply_one(|row| row.peg = Some(1.5)));
        assert!(!apply_one(|row| row.peg = Some(0.009)));
        assert!(!apply_one(|row| row.peg = Some(1.501)));

        assert!(apply_one(|row| row.pfcf = Some(1.0)));
        assert!(apply_one(|row| row.pfcf = Some(180.0)));
        assert!(!apply_one(|row| row.pfcf = Some(0.99)));
        assert!(!apply_one(|row| row.pfcf = Some(180.01)));

        assert!(apply_one(|row| row.roe = Some(0.15)));
        assert!(!apply_one(|row| row.roe = Some(0.1499)));

        assert!(apply_one(|row| row.roic = Some(0.10)));
        assert!(!apply_one(|row| row.roic = Some(0.0999)));

        assert!(apply_one(|row| row.profit_margin = Some(0.10)));
        assert!(!apply_one(|row| row.profit_margin = Some(0.0999)));

        assert!(apply_one(|row| {
            row.eps_past_5y = Some(0.10);
            row.sales_past_5y = Some(0.10);
        }));
        assert!(!apply_one(|row| row.eps_past_5y = Some(0.099)));
        assert!(!apply_one(|row| row.sales_past_5y = Some(0.099)));
        assert!(!apply_one(|row| {
            row.eps_past_5y = None;
            row.sales_past_5y = None;
        }));

        assert!(apply_one(|row| row.debt_equity = Some(1.5)));
        assert!(!apply_one(|row| row.debt_equity = Some(1.5001)));
    }

    #[test]
    fn checks_technical_boundaries() {
        assert!(apply_one(|row| row.roc125 = Some(0.10)));
        assert!(apply_one(|row| row.roc125 = Some(4.0)));
        assert!(!apply_one(|row| row.roc125 = Some(0.099)));
        assert!(!apply_one(|row| row.roc125 = Some(4.001)));

        assert!(apply_one(|row| row.roc20 = Some(-0.149)));
        assert!(!apply_one(|row| row.roc20 = Some(-0.15)));

        assert!(apply_one(|row| row.ema200_distance = Some(0.0001)));
        assert!(!apply_one(|row| row.ema200_distance = Some(0.0)));
        assert!(!apply_one(|row| row.ema200_distance = Some(-0.0001)));

        assert!(apply_one(|row| row.rsi14 = Some(35.0)));
        assert!(apply_one(|row| row.rsi14 = Some(75.0)));
        assert!(!apply_one(|row| row.rsi14 = Some(34.99)));
        assert!(!apply_one(|row| row.rsi14 = Some(75.01)));
    }

    #[test]
    fn fails_when_any_input_is_missing() {
        assert!(!apply_one(|row| row.roe = None));
        assert!(!apply_one(|row| row.debt_equity = None));
        assert!(!apply_one(|row| row.forward_pe = None));
        assert!(!apply_one(|row| row.peg = None));
        assert!(!apply_one(|row| row.pfcf = None));
        assert!(!apply_one(|row| row.roic = None));
        assert!(!apply_one(|row| row.profit_margin = None));
        assert!(!apply_one(|row| row.ema200_distance = None));
        assert!(!apply_one(|row| row.roc20 = None));
        assert!(!apply_one(|row| row.rsi14 = None));
        assert!(!apply_one(|row| row.roc125 = None));
    }

    #[test]
    fn requires_both_fundamental_and_technical() {
        assert!(!apply_one(|row| row.rsi14 = Some(90.0)));
        assert!(!apply_one(|row| row.market_cap = Some(1_000_000_000.0)));
    }

    #[test]
    fn ranks_roc125_against_the_whole_market() {
        // Six rows that only differ on ROC125. Percentiles are 0, 20, 40, 60,
        // 80, 100, so only the last three clear the 60th percentile.
        let rows: Vec<StockRow> = [0.01, 0.03, 0.06, 0.10, 0.20, 0.50]
            .iter()
            .enumerate()
            .map(|(index, roc125)| StockRow {
                roc125: Some(*roc125),
                ..all_pass_stock(&format!("T{index}"))
            })
            .collect();

        assert_eq!(apply(&rows), vec![false, false, false, true, true, true]);
    }

    #[test]
    fn fails_a_row_with_no_inputs_at_all() {
        let rows = vec![StockRow {
            ps: Some(9.0),
            ..StockRow::with_ticker("EMPTY")
        }];

        assert_eq!(apply(&rows), vec![false]);
    }
}
