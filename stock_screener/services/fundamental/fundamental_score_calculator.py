import pandas as pd

from stock_screener.services.common.percentile_scorer import percentile_score
from stock_screener.services.common.series_normalizer import to_numeric_series
from stock_screener.utils.screener_rules import (
    FUNDAMENTAL_SCORE_COLUMN,
    MARKET_CAP_COLUMN,
)


SCORE_COLUMN = FUNDAMENTAL_SCORE_COLUMN
SCORE_METRICS = [
    (MARKET_CAP_COLUMN, True, 0.05, "Market Cap Score"),
    ("EPS Past 5Y", True, 0.18, "EPS Past 5Y Score"),
    ("Sales Past 5Y", True, 0.1, "Sales Past 5Y Score"),
    ("ROE", True, 0.12, "ROE Score"),
    ("ROIC", True, 0.15, "ROIC Score"),
    ("Profit Margin", True, 0.08, "Profit Margin Score"),
    ("Forward P/E", False, 0.05, "Forward P/E Score"),
    ("PEG", False, 0.12, "PEG Score"),
    ("P/S", False, 0.02, "P/S Score"),
    ("P/FCF", False, 0.08, "P/FCF Score"),
    ("Debt/Equity", False, 0.05, "Debt/Equity Score"),
]
SCORE_WEIGHTS = {metric: weight for metric, _, weight, _ in SCORE_METRICS}


class FundamentalScoreCalculator:
    def add_score(
        self,
        data: pd.DataFrame,
        columns: list[str],
    ) -> pd.DataFrame:
        if data.empty:
            return data

        scored_data = data.copy()
        score_parts = []
        for column, higher_is_better, weight, score_column in SCORE_METRICS:
            if column in scored_data.columns:
                raw_score = self.score_column(scored_data[column], higher_is_better)
                scored_data[score_column] = raw_score.round(2)
                score_parts.append(raw_score * weight)

        if not score_parts:
            scored_data[SCORE_COLUMN] = 0.0
            return scored_data

        score_frame = pd.concat(score_parts, axis=1)
        scored_data[SCORE_COLUMN] = score_frame.sum(axis=1).round(2)
        sorted_data = scored_data.sort_values(
            by=SCORE_COLUMN,
            ascending=False,
            ignore_index=True,
        )
        selected_columns = [column for column in columns if column in sorted_data.columns]
        return sorted_data.loc[:, selected_columns]

    def score_column(
        self,
        data: pd.Series,
        higher_is_better: bool,
    ) -> pd.Series:
        numeric_data = to_numeric_series(data)
        return percentile_score(
            numeric_data,
            ascending=higher_is_better,
        )
