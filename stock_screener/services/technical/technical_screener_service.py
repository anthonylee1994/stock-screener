import logging
import pandas as pd

from stock_screener.services.technical.technical_indicator_calculator import TechnicalIndicatorCalculator
from stock_screener.services.technical.technical_price_client import TechnicalPriceClient
from stock_screener.services.technical.technical_score_calculator import (
    DEFAULT_COLUMNS,
    TechnicalScoreCalculator,
)
from stock_screener.utils.ticker_normalizer import normalize_tickers


logger = logging.getLogger(__name__)


class TechnicalScreenerService:
    def __init__(
        self,
        price_client: TechnicalPriceClient | None = None,
        indicator_calculator: TechnicalIndicatorCalculator | None = None,
        score_calculator: TechnicalScoreCalculator | None = None,
    ):
        self.price_client = price_client or TechnicalPriceClient()
        self.indicator_calculator = indicator_calculator or TechnicalIndicatorCalculator(
            price_client=self.price_client
        )
        self.score_calculator = score_calculator or TechnicalScoreCalculator()

    def run(self, tickers: list[str]) -> pd.DataFrame:
        normalized_tickers = normalize_tickers(tickers)
        if not normalized_tickers:
            return pd.DataFrame(columns=DEFAULT_COLUMNS)

        try:
            price_data = self.price_client.download_price_data(normalized_tickers)
        except Exception as error:
            logger.exception(
                "獲取技術面價格資料失敗 tickers=%s: %s",
                len(normalized_tickers),
                error,
            )
            return pd.DataFrame(columns=DEFAULT_COLUMNS)
        indicator_data = self.indicator_calculator.calculate_indicators(
            price_data,
            tickers=normalized_tickers,
        )
        if indicator_data.empty:
            return pd.DataFrame(columns=DEFAULT_COLUMNS)
        scored_result = self.score_calculator.add_scores(indicator_data)
        return scored_result.copy()


technical_screener_service = TechnicalScreenerService()
