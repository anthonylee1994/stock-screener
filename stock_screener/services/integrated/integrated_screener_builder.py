import logging
import time

import pandas as pd

from stock_screener.services.common.score_curver import curve_score
from stock_screener.services.common.series_normalizer import to_numeric_series
from stock_screener.services.fundamental.fundamental_score_calculator import SCORE_COLUMN as FUNDAMENTAL_SCORE_COLUMN
from stock_screener.services.fundamental.fundamental_screener_service import fundamental_screener_service
from stock_screener.services.integrated.potential_stock_filter import PotentialStockFilter
from stock_screener.services.technical.technical_score_calculator import SCORE_COLUMN as TECHNICAL_SCORE_COLUMN
from stock_screener.services.technical.technical_screener_service import (
    technical_screener_service,
)
from stock_screener.utils.screener_rules import (
    POTENTIAL_STOCK_COLUMN,
    TOTAL_SCORE_COLUMN,
)


SCORE_COLUMN = TOTAL_SCORE_COLUMN
FUNDAMENTAL_SCORE_WEIGHT = 0.75
TECHNICAL_SCORE_WEIGHT = 0.25
logger = logging.getLogger(__name__)


class IntegratedScreenerBuilder:
    def __init__(
        self,
        fundamental_service=fundamental_screener_service,
        technical_service=technical_screener_service,
        potential_stock_filter=None,
    ):
        self.fundamental_service = fundamental_service
        self.technical_service = technical_service
        self.potential_stock_filter = potential_stock_filter or PotentialStockFilter()

    def build(self, limit: int) -> pd.DataFrame:
        started_at = time.monotonic()
        logger.info("開始建立完整篩選器資料 limit=%s", limit)

        fundamental_data = self.get_fundamental_data(limit)
        if fundamental_data.empty or "Ticker" not in fundamental_data.columns:
            logger.warning(
                "完整篩選器資料冇 ticker rows=%s elapsed=%.2fs",
                len(fundamental_data),
                time.monotonic() - started_at,
            )
            return fundamental_data.copy()

        technical_data = self.get_technical_data(fundamental_data)
        if technical_data.empty:
            full_data = self.add_total_score(fundamental_data)
        else:
            full_data = self.add_total_score(
                fundamental_data.merge(technical_data, on="Ticker", how="left")
            )

        full_data[POTENTIAL_STOCK_COLUMN] = self.potential_stock_filter.apply(full_data)

        logger.info(
            "完整篩選器資料已建立 limit=%s rows=%s elapsed=%.2fs",
            limit,
            len(full_data),
            time.monotonic() - started_at,
        )
        return full_data.copy()

    def get_fundamental_data(self, limit: int) -> pd.DataFrame:
        logger.info("獲取基本面篩選器資料 limit=%s", limit)
        started_at = time.monotonic()
        data = self.fundamental_service.run(limit=limit)
        logger.info(
            "已獲取基本面篩選器資料 rows=%s elapsed=%.2fs",
            len(data),
            time.monotonic() - started_at,
        )
        return data

    def get_technical_data(self, fundamental_data: pd.DataFrame) -> pd.DataFrame:
        tickers = fundamental_data["Ticker"].dropna().astype(str).tolist()
        logger.info("獲取技術面篩選器資料 tickers=%s", len(tickers))
        started_at = time.monotonic()
        data = self.technical_service.run(tickers)
        logger.info(
            "已獲取技術面篩選器資料 rows=%s elapsed=%.2fs",
            len(data),
            time.monotonic() - started_at,
        )
        return data

    def add_total_score(self, data: pd.DataFrame) -> pd.DataFrame:
        if data.empty:
            return data

        scored_data = data.copy()
        if (
            FUNDAMENTAL_SCORE_COLUMN not in scored_data.columns
            or TECHNICAL_SCORE_COLUMN not in scored_data.columns
        ):
            scored_data[SCORE_COLUMN] = pd.NA
            return scored_data

        fundamental_score = to_numeric_series(
            scored_data[FUNDAMENTAL_SCORE_COLUMN])
        technical_score = to_numeric_series(
            scored_data[TECHNICAL_SCORE_COLUMN])
        valid_score_mask = fundamental_score.notna() & technical_score.notna()
        score = (
            (fundamental_score * FUNDAMENTAL_SCORE_WEIGHT)
            + (technical_score * TECHNICAL_SCORE_WEIGHT)
        ).where(valid_score_mask)
        scored_data[SCORE_COLUMN] = curve_score(score).round(2)
        return scored_data
