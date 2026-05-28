import pandas as pd

from stock_screener.services.common.score_curver import curve_score
from stock_screener.services.common.percentile_scorer import percentile_score
from stock_screener.services.common.series_normalizer import to_numeric_series
from stock_screener.utils.screener_rules import (
    FUNDAMENTAL_SCORE_COLUMN,
    MARKET_CAP_COLUMN,
)


SCORE_COLUMN = FUNDAMENTAL_SCORE_COLUMN
SCORE_METRICS = [
    (MARKET_CAP_COLUMN, True, 0, "Market Cap Score"),
    ("EPS Past 5Y", True, 0.18, "EPS Past 5Y Score"),
    ("Sales Past 5Y", True, 0.08, "Sales Past 5Y Score"),
    ("ROE", True, 0.13, "ROE Score"),
    ("ROIC", True, 0.2, "ROIC Score"),
    ("Profit Margin", True, 0.06, "Profit Margin Score"),
    ("Forward P/E", False, 0.05, "Forward P/E Score"),
    ("PEG", False, 0.17, "PEG Score"),
    ("P/S", False, 0.03, "P/S Score"),
    ("P/FCF", False, 0.07, "P/FCF Score"),
    ("Debt/Equity", False, 0.03, "Debt/Equity Score"),
]
SCORE_WEIGHTS = {metric: weight for metric, _, weight, _ in SCORE_METRICS}
MIN_SECTOR_SCORE_SAMPLE_SIZE = 5
CORE_SCORE_METRICS = ("ROIC", "EPS Past 5Y", "PEG")
MIN_CORE_SCORE_METRIC_COUNT = 2
INSUFFICIENT_CORE_SCORE_CAP = 60.0
CORE_SCORE_COLUMNS = ("ROIC Score", "EPS Past 5Y Score", "PEG Score")
MIN_CORE_AVERAGE_SCORE = 70.0
WEAK_CORE_SCORE_CAP = 75.0


class FundamentalScoreCalculator:
    def add_score(
        self,
        data: pd.DataFrame,
        columns: list[str],
    ) -> pd.DataFrame:
        if data.empty:
            return data

        scored_data = data.copy()
        sectors = scored_data.get("Sector")
        score_parts = []
        for column, higher_is_better, weight, score_column in SCORE_METRICS:
            if column in scored_data.columns:
                raw_score = self.score_column(
                    scored_data[column],
                    higher_is_better,
                    sectors=sectors,
                )
                scored_data[score_column] = raw_score.round(2)
                score_parts.append(raw_score * weight)

        if not score_parts:
            scored_data[SCORE_COLUMN] = 0.0
            return scored_data

        score_frame = pd.concat(score_parts, axis=1)
        scored_data[SCORE_COLUMN] = score_frame.sum(axis=1).round(2)
        scored_data[SCORE_COLUMN] = curve_score(scored_data[SCORE_COLUMN]).round(2)
        scored_data[SCORE_COLUMN] = self.apply_core_metric_guardrail(scored_data)
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
        sectors: pd.Series | None = None,
    ) -> pd.Series:
        numeric_data = to_numeric_series(data)
        global_score = percentile_score(
            numeric_data,
            ascending=higher_is_better,
        )
        if sectors is None:
            return global_score

        sector_values = sectors.reindex(data.index).map(self.normalize_sector)
        sector_score = pd.Series(pd.NA, index=data.index, dtype="Float64")
        for sector in sector_values.dropna().unique():
            sector_index = sector_values[sector_values == sector].index
            sector_data = numeric_data.loc[sector_index]
            if sector_data.notna().sum() < MIN_SECTOR_SCORE_SAMPLE_SIZE:
                continue

            sector_score.loc[sector_index] = percentile_score(
                sector_data,
                ascending=higher_is_better,
            )

        return sector_score.astype("float64").fillna(global_score)

    def normalize_sector(self, value):
        if pd.isna(value):
            return pd.NA
        sector = str(value).strip()
        if not sector:
            return pd.NA
        return sector

    def apply_core_metric_guardrail(self, data: pd.DataFrame) -> pd.Series:
        core_metric_data = pd.DataFrame(index=data.index)
        for column in CORE_SCORE_METRICS:
            if column in data.columns:
                core_metric_data[column] = to_numeric_series(data[column])
            else:
                core_metric_data[column] = pd.NA

        core_metric_count = core_metric_data.notna().sum(axis=1)
        guarded_score = data[SCORE_COLUMN].where(
            core_metric_count >= MIN_CORE_SCORE_METRIC_COUNT,
            data[SCORE_COLUMN].clip(upper=INSUFFICIENT_CORE_SCORE_CAP),
        )
        core_average_score = self.calculate_core_average_score(data)
        return guarded_score.where(
            core_average_score >= MIN_CORE_AVERAGE_SCORE,
            guarded_score.clip(upper=WEAK_CORE_SCORE_CAP),
        )

    def calculate_core_average_score(self, data: pd.DataFrame) -> pd.Series:
        core_scores = pd.DataFrame(index=data.index)
        for column in CORE_SCORE_COLUMNS:
            if column in data.columns:
                core_scores[column] = to_numeric_series(data[column])
            else:
                core_scores[column] = pd.NA
        return core_scores.mean(axis=1).fillna(0.0)
