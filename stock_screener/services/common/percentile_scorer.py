import pandas as pd


def percentile_score(data: pd.Series, ascending: bool) -> pd.Series:
    valid_count = int(data.notna().sum())
    if valid_count == 0:
        return pd.Series(0.0, index=data.index)
    if valid_count == 1:
        return data.notna().astype(float) * 100

    ranks = data.rank(method="average", ascending=ascending)
    scores = ((ranks - 1) / (valid_count - 1)) * 100
    return scores.fillna(0.0)
