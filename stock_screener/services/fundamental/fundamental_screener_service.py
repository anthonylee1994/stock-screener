import pandas as pd

from stock_screener.services.fundamental.fundamental_data_normalizer import FundamentalDataNormalizer
from stock_screener.services.fundamental.fundamental_score_calculator import (
    SCORE_COLUMN,
    FundamentalScoreCalculator,
)
from stock_screener.services.fundamental.fundamental_screener_client import FundamentalScreenerClient
from stock_screener.utils.screener_rules import (
    MARKET_CAP_COLUMN,
    POTENTIAL_STOCK_COLUMN,
    VOLUME_COLUMN,
)


DEFAULT_COLUMNS = [
    "Ticker",
    "Company",
    "Sector",
    MARKET_CAP_COLUMN,
    "Market Cap Score",
    "Forward P/E",
    "Forward P/E Score",
    "PEG",
    "PEG Score",
    "P/S",
    "P/S Score",
    "P/FCF",
    "P/FCF Score",
    "EPS Past 5Y",
    "EPS Past 5Y Score",
    "Sales Past 5Y",
    "Sales Past 5Y Score",
    "EPS Quarter Over Quarter",
    "Sales Quarter Over Quarter",
    "ROE",
    "ROE Score",
    "ROIC",
    "ROIC Score",
    "Profit Margin",
    "Profit Margin Score",
    "Gross Margin",
    "Operating Margin",
    "Debt/Equity",
    "Debt/Equity Score",
    "Short Interest",
    "52W High",
    "Target Price",
    POTENTIAL_STOCK_COLUMN,
    "Price",
    "Change",
    VOLUME_COLUMN,
    SCORE_COLUMN,
]


class FundamentalScreenerService:
    def __init__(
        self,
        client: FundamentalScreenerClient | None = None,
        normalizer: FundamentalDataNormalizer | None = None,
        score_calculator: FundamentalScoreCalculator | None = None,
    ):
        self.client = client or FundamentalScreenerClient()
        self.normalizer = normalizer or FundamentalDataNormalizer()
        self.score_calculator = score_calculator or FundamentalScoreCalculator()

    def run(
        self,
        limit: int,
    ) -> pd.DataFrame:
        result = self.client.fetch(limit)
        cleaned_result = self.normalizer.remove_invalid_rows(result)
        normalized_result = self.normalizer.normalize(cleaned_result)
        scored_result = self.score_calculator.add_score(
            normalized_result,
            columns=DEFAULT_COLUMNS,
        )
        return scored_result.copy()


fundamental_screener_service = FundamentalScreenerService()
