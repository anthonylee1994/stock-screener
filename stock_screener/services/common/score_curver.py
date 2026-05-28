import pandas as pd

from stock_screener.services.common.series_normalizer import to_numeric_series


def curve_score(score: pd.Series) -> pd.Series:
    numeric_score = to_numeric_series(score).where(score.notna())
    valid_score = numeric_score.dropna()
    if valid_score.empty:
        return pd.Series(pd.NA, index=score.index, dtype="Float64")
    if len(valid_score) == 1:
        return pd.Series(100.0, index=valid_score.index).reindex(score.index)

    min_score = valid_score.min()
    max_score = valid_score.max()
    if min_score == max_score:
        return pd.Series(100.0, index=valid_score.index).reindex(score.index)

    return ((numeric_score - min_score) / (max_score - min_score)) * 100
