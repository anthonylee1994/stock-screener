import pandas as pd

from stock_screener.services.common.score_curver import curve_score
from stock_screener.services.common.percentile_scorer import percentile_score
from stock_screener.utils.screener_rules import TECHNICAL_SCORE_COLUMN


SCORE_COLUMN = TECHNICAL_SCORE_COLUMN
DEFAULT_COLUMNS = [
    "Ticker",
    "Quote Price",
    "Quote Change",
    "Quote Change Percent",
    "Quote Volume",
    "Long Term Score",
    "Mid Term Score",
    "Short Term Score",
    SCORE_COLUMN,
    "EMA200Distance",
    "ROC125",
    "EMA50Distance",
    "ROC20",
    "PPO Slope3",
    "RSI14",
]
TERM_SCORES = {
    "Long Term Score": ("EMA200Distance", "ROC125", 0.6),
    "Mid Term Score": ("EMA50Distance", "ROC20", 0.3),
    "Short Term Score": ("PPO Slope3", "RSI14", 0.1),
}


class TechnicalScoreCalculator:
    def add_scores(self, data: pd.DataFrame) -> pd.DataFrame:
        scored_data = data.copy()
        score_parts = []
        for score_column, (first_column, second_column, weight) in TERM_SCORES.items():
            scored_data[score_column] = (
                percentile_score(scored_data[first_column], ascending=True)
                + percentile_score(scored_data[second_column], ascending=True)
            ) / 2
            score_parts.append(scored_data[score_column] * weight)

        scored_data[SCORE_COLUMN] = curve_score(sum(score_parts))

        score_columns = [*TERM_SCORES, SCORE_COLUMN]
        scored_data[score_columns] = scored_data[score_columns].round(2)
        return scored_data.loc[:, DEFAULT_COLUMNS].sort_values(
            by=SCORE_COLUMN,
            ascending=False,
            ignore_index=True,
        )
