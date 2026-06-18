import pandas as pd

from stock_screener.services.common.percentile_scorer import percentile_score
from stock_screener.services.common.series_normalizer import to_numeric_series
from stock_screener.utils.screener_rules import MARKET_CAP_COLUMN


# 基本面硬篩（護城河 + 增長）
MIN_POTENTIAL_MARKET_CAP = 15_000_000_000      # $15B — 流動性 + mega/large cap
MIN_POTENTIAL_ROE = 0.18                        # 18% — 資本回報 / 護城河
MIN_POTENTIAL_EPS_PAST_5Y = 0.15                # 15% — 增長（EPS 分支）
MIN_POTENTIAL_SALES_PAST_5Y = 0.18              # 18% — 增長（Sales 分支）
MAX_POTENTIAL_DEBT_EQUITY = 1.5                 # 資產負債表穩健
MIN_POTENTIAL_GROSS_MARGIN = 0.40               # 40% — 定價力 / 護城河 proxy

# 技術確認（動量 + 趨勢）
MIN_POTENTIAL_ROC125_PERCENTILE = 60            # 全市場 60th percentile（用 rank 避免 raw ROC noise）
MIN_POTENTIAL_RSI = 40.0
MAX_POTENTIAL_RSI = 78.0                        # 唔超買唔超賣


class PotentialStockFilter:
    """篩「基本面 + 技術」雙強候選股。

    基本面硬篩同技術確認兩組條件要同時通過，先會標記做 ``Potential Stock``。
    入參係已經 merge 完 fundamental + technical 嘅 DataFrame。
    """

    def apply(self, data: pd.DataFrame) -> pd.Series:
        market_cap = self.metric(data, MARKET_CAP_COLUMN)
        roe = self.metric(data, "ROE")
        eps_past_5y = self.metric(data, "EPS Past 5Y")
        sales_past_5y = self.metric(data, "Sales Past 5Y")
        debt_equity = self.metric(data, "Debt/Equity")
        gross_margin = self.metric(data, "Gross Margin")
        roc125 = self.metric(data, "ROC125")
        ema200_distance = self.metric(data, "EMA200Distance")
        rsi14 = self.metric(data, "RSI14")

        roc125_percentile = percentile_score(roc125, ascending=True)

        fundamental_pass = (
            (market_cap >= MIN_POTENTIAL_MARKET_CAP)
            & (roe >= MIN_POTENTIAL_ROE)
            & (
                (eps_past_5y >= MIN_POTENTIAL_EPS_PAST_5Y)
                | (sales_past_5y >= MIN_POTENTIAL_SALES_PAST_5Y)
            )
            & (debt_equity < MAX_POTENTIAL_DEBT_EQUITY)
            & (gross_margin >= MIN_POTENTIAL_GROSS_MARGIN)
        )
        technical_pass = (
            (roc125_percentile >= MIN_POTENTIAL_ROC125_PERCENTILE)
            & (ema200_distance > 0)
            & (rsi14 >= MIN_POTENTIAL_RSI)
            & (rsi14 <= MAX_POTENTIAL_RSI)
        )

        return (fundamental_pass & technical_pass).fillna(False).astype(bool)

    def metric(self, data: pd.DataFrame, column: str) -> pd.Series:
        if column not in data.columns:
            return pd.Series(pd.NA, index=data.index, dtype="Float64")
        return to_numeric_series(data[column])
