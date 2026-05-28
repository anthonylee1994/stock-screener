import pandas as pd


def to_numeric_series(data: pd.Series) -> pd.Series:
    text_data = data.astype(str)
    text_data = text_data.str.replace("%", "", regex=False)
    text_data = text_data.str.replace(",", "", regex=False)
    text_data = text_data.replace({"-": pd.NA, "nan": pd.NA, "None": pd.NA})
    return pd.to_numeric(text_data, errors="coerce")
