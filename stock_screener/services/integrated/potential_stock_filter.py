import pandas as pd

from stock_screener.services.common.percentile_scorer import percentile_score
from stock_screener.services.common.series_normalizer import to_numeric_series
from stock_screener.utils.screener_rules import MARKET_CAP_COLUMN


# 基本面硬篩（護城河 + 增長）
MIN_POTENTIAL_MARKET_CAP = 10_000_000_000  # $10B — large cap universe
MIN_POTENTIAL_VOLUME = 1_000_000  # Finviz Avg Volume > 1M proxy
MIN_POTENTIAL_FORWARD_PE = 1.0  # 正盈利，避免 loss-making 當 cheap
MAX_POTENTIAL_FORWARD_PE = 60.0  # 避免估值太離地
MIN_POTENTIAL_PEG = 0.01  # 正增長估值
MAX_POTENTIAL_PEG = 1.5  # growth-adjusted valuation sanity check
MIN_POTENTIAL_PFCF = 1.0  # 正 FCF valuation
MAX_POTENTIAL_PFCF = 180.0  # 保留高質 compounder，但排除極端值
MIN_POTENTIAL_ROE = 0.15  # 15% — 資本回報 / 護城河
MIN_POTENTIAL_ROIC = 0.10  # 10% — capital efficiency
MIN_POTENTIAL_PROFIT_MARGIN = 0.10  # 10% — 盈利質素
MIN_POTENTIAL_EPS_PAST_5Y = 0.10  # 10% — EPS 成長
MIN_POTENTIAL_SALES_PAST_5Y = 0.10  # 10% — Sales 成長
MAX_POTENTIAL_DEBT_EQUITY = 1.5  # 資產負債表穩健

# 技術確認（動量 + 趨勢）
MIN_POTENTIAL_ROC125 = 0.10  # Performance Half Year up 10% proxy
MAX_POTENTIAL_ROC125 = 4.00  # 避免 split / spin-off artifact
MIN_POTENTIAL_ROC125_PERCENTILE = (
    60  # 全市場 60th percentile（用 rank 避免 raw ROC noise）
)
MIN_POTENTIAL_ROC20 = -0.15  # Performance Month not worse than -15%
MIN_POTENTIAL_RSI = 35.0
MAX_POTENTIAL_RSI = 75.0  # 唔超買唔超賣


class PotentialStockFilter:
    """篩「基本面 + 技術」雙強候選股。

    基本面硬篩同技術確認兩組條件要同時通過，先會標記做 ``Potential Stock``。
    入參係已經 merge 完 fundamental + technical 嘅 DataFrame。
    """

    def apply(self, data: pd.DataFrame) -> pd.Series:
        market_cap = self.metric(data, MARKET_CAP_COLUMN)
        volume = self.metric(data, "Volume")
        forward_pe = self.metric(data, "Forward P/E")
        peg = self.metric(data, "PEG")
        pfcf = self.metric(data, "P/FCF")
        roe = self.metric(data, "ROE")
        roic = self.metric(data, "ROIC")
        profit_margin = self.metric(data, "Profit Margin")
        eps_past_5y = self.metric(data, "EPS Past 5Y")
        sales_past_5y = self.metric(data, "Sales Past 5Y")
        debt_equity = self.metric(data, "Debt/Equity")
        roc125 = self.metric(data, "ROC125")
        roc20 = self.metric(data, "ROC20")
        ema200_distance = self.metric(data, "EMA200Distance")
        rsi14 = self.metric(data, "RSI14")

        roc125_percentile = percentile_score(roc125, ascending=True)

        fundamental_pass = (
            (market_cap >= MIN_POTENTIAL_MARKET_CAP)
            & (volume >= MIN_POTENTIAL_VOLUME)
            & (forward_pe >= MIN_POTENTIAL_FORWARD_PE)
            & (forward_pe <= MAX_POTENTIAL_FORWARD_PE)
            & (peg >= MIN_POTENTIAL_PEG)
            & (peg <= MAX_POTENTIAL_PEG)
            & (pfcf >= MIN_POTENTIAL_PFCF)
            & (pfcf <= MAX_POTENTIAL_PFCF)
            & (roe >= MIN_POTENTIAL_ROE)
            & (roic >= MIN_POTENTIAL_ROIC)
            & (profit_margin >= MIN_POTENTIAL_PROFIT_MARGIN)
            & (eps_past_5y >= MIN_POTENTIAL_EPS_PAST_5Y)
            & (sales_past_5y >= MIN_POTENTIAL_SALES_PAST_5Y)
            & (debt_equity <= MAX_POTENTIAL_DEBT_EQUITY)
        )
        technical_pass = (
            (roc125 >= MIN_POTENTIAL_ROC125)
            & (roc125 <= MAX_POTENTIAL_ROC125)
            & (roc125_percentile >= MIN_POTENTIAL_ROC125_PERCENTILE)
            & (roc20 > MIN_POTENTIAL_ROC20)
            & (ema200_distance > 0)
            & (rsi14 >= MIN_POTENTIAL_RSI)
            & (rsi14 <= MAX_POTENTIAL_RSI)
        )

        return (fundamental_pass & technical_pass).fillna(False).astype(bool)

    def metric(self, data: pd.DataFrame, column: str) -> pd.Series:
        if column not in data.columns:
            return pd.Series(pd.NA, index=data.index, dtype="Float64")
        return to_numeric_series(data[column])
