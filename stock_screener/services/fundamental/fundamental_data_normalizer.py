import pandas as pd

from stock_screener.services.common.series_normalizer import to_numeric_series
from stock_screener.utils.screener_rules import MARKET_CAP_COLUMN, VOLUME_COLUMN


COLUMN_ALIASES = {
    "Market Cap.": MARKET_CAP_COLUMN,
    "Fwd P/E": "Forward P/E",
    "P/Free Cash Flow": "P/FCF",
    "P/FCF": "P/FCF",
    "EPS growth past 5 years": "EPS Past 5Y",
    "EPS past 5Y": "EPS Past 5Y",
    "Sales growth past 5 years": "Sales Past 5Y",
    "Sales past 5Y": "Sales Past 5Y",
    "Return on Equity": "ROE",
    "Return on Investments": "ROIC",
    "ROI": "ROIC",
    "Total Debt/Equity": "Debt/Equity",
    "Debt/Eq": "Debt/Equity",
    "Net Profit Margin": "Profit Margin",
    "Profit M": "Profit Margin",
}
NUMERIC_COLUMNS = [
    MARKET_CAP_COLUMN,
    "Forward P/E",
    "PEG",
    "P/S",
    "P/FCF",
    "ROE",
    "ROIC",
    "Profit Margin",
    "Debt/Equity",
    "Price",
    "Change",
    VOLUME_COLUMN,
]
PERCENT_COLUMNS = ["EPS Past 5Y", "Sales Past 5Y", "ROIC"]
NON_SCORE_METRIC_COLUMNS = [
    MARKET_CAP_COLUMN,
    "Forward P/E",
    "PEG",
    "P/S",
    "P/FCF",
    "EPS Past 5Y",
    "Sales Past 5Y",
    "ROE",
    "ROIC",
    "Profit Margin",
    "Debt/Equity",
]


class FundamentalDataNormalizer:
    def normalize(self, data: pd.DataFrame) -> pd.DataFrame:
        if data.empty:
            return data

        normalized_data = data.copy()
        normalized_data = normalized_data.rename(columns=COLUMN_ALIASES)
        for column in NUMERIC_COLUMNS:
            if column in normalized_data.columns:
                normalized_data[column] = to_numeric_series(normalized_data[column])
        for column in PERCENT_COLUMNS:
            if column in normalized_data.columns:
                normalized_data[column] = (
                    to_numeric_series(normalized_data[column]) / 100
                )
        for column in NON_SCORE_METRIC_COLUMNS:
            if column in normalized_data.columns:
                normalized_data[column] = normalized_data[column].round(4)
        return normalized_data

    def remove_invalid_rows(self, data: pd.DataFrame) -> pd.DataFrame:
        if data.empty:
            return data

        cleaned_data = data.copy()
        if "Ticker" in cleaned_data.columns:
            cleaned_data = cleaned_data[
                cleaned_data["Ticker"].fillna("").astype(str).str.strip() != ""
            ]
        return cleaned_data.reset_index(drop=True)
