import pandas as pd

from stock_screener.services.common.series_normalizer import to_numeric_series
from stock_screener.utils.screener_rules import (
    MARKET_CAP_COLUMN,
    TARGET_PRICE_UPSIDE_COLUMN,
    VOLUME_COLUMN,
)


COLUMN_ALIASES = {
    "Market Cap.": MARKET_CAP_COLUMN,
    "Fwd P/E": "Forward P/E",
    "P/Free Cash Flow": "P/FCF",
    "P/FCF": "P/FCF",
    "EPS growth past 5 years": "EPS Past 5Y",
    "EPS past 5Y": "EPS Past 5Y",
    "EPS Q/Q": "EPS Quarter Over Quarter",
    "Sales growth past 5 years": "Sales Past 5Y",
    "Sales past 5Y": "Sales Past 5Y",
    "Sales Q/Q": "Sales Quarter Over Quarter",
    "Return on Equity": "ROE",
    "Return on Investments": "ROIC",
    "ROI": "ROIC",
    "Total Debt/Equity": "Debt/Equity",
    "Debt/Eq": "Debt/Equity",
    "Net Profit Margin": "Profit Margin",
    "Profit M": "Profit Margin",
    "Gross M": "Gross Margin",
    "Oper M": "Operating Margin",
    "Float Short": "Short Interest",
    "Short Float": "Short Interest",
    "SMA200": "200-Day Simple Moving Average",
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
    "Gross Margin",
    "Operating Margin",
    "Debt/Equity",
    "Short Interest",
    "200-Day Simple Moving Average",
    "52W High",
    "Target Price",
    "Price",
    TARGET_PRICE_UPSIDE_COLUMN,
    "Change",
    VOLUME_COLUMN,
]
PERCENT_COLUMNS = [
    "EPS Past 5Y",
    "EPS Quarter Over Quarter",
    "Sales Past 5Y",
    "Sales Quarter Over Quarter",
    "ROE",
    "ROIC",
    "Profit Margin",
    "Gross Margin",
    "Operating Margin",
    "Short Interest",
    "200-Day Simple Moving Average",
    "52W High",
]
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
    "Gross Margin",
    "Operating Margin",
    "Debt/Equity",
    "EPS Quarter Over Quarter",
    "Sales Quarter Over Quarter",
    "Short Interest",
    "200-Day Simple Moving Average",
    "52W High",
    "Target Price",
    TARGET_PRICE_UPSIDE_COLUMN,
]


class FundamentalDataNormalizer:
    def normalize(self, data: pd.DataFrame) -> pd.DataFrame:
        if data.empty:
            return data

        normalized_data = data.copy()
        normalized_data = normalized_data.rename(columns=COLUMN_ALIASES)
        raw_data = normalized_data.copy()
        for column in NUMERIC_COLUMNS:
            if column in normalized_data.columns:
                normalized_data[column] = to_numeric_series(
                    normalized_data[column])
        for column in PERCENT_COLUMNS:
            if column in normalized_data.columns:
                normalized_data[column] = self.normalize_percent_series(
                    raw_data[column])
        if {"Target Price", "Price"}.issubset(normalized_data.columns):
            normalized_data[TARGET_PRICE_UPSIDE_COLUMN] = (
                (normalized_data["Target Price"] - normalized_data["Price"])
                / normalized_data["Price"].where(normalized_data["Price"] != 0)
            )
        for column in NON_SCORE_METRIC_COLUMNS:
            if column in normalized_data.columns:
                normalized_data[column] = normalized_data[column].round(4)
        return normalized_data

    def normalize_percent_series(self, data: pd.Series) -> pd.Series:
        numeric_data = to_numeric_series(data)
        has_percent_symbol = data.astype(str).str.contains("%", regex=False)
        return numeric_data.where(~has_percent_symbol, numeric_data / 100)

    def remove_invalid_rows(self, data: pd.DataFrame) -> pd.DataFrame:
        if data.empty:
            return data

        cleaned_data = data.copy()
        if "Ticker" in cleaned_data.columns:
            cleaned_data = cleaned_data[
                cleaned_data["Ticker"].fillna("").astype(str).str.strip() != ""
            ]
        return cleaned_data.reset_index(drop=True)
