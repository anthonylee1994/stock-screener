import pandas as pd

from stock_screener.utils.screener_rules import (
    CHANGE_PERCENT_COLUMN,
    FUNDAMENTAL_SCORE_COLUMN,
    MARKET_CAP_COLUMN,
    MIN_VOLUME,
    POTENTIAL_STOCK_COLUMN,
    TARGET_PRICE_UPSIDE_COLUMN,
    TECHNICAL_SCORE_COLUMN,
    TOTAL_SCORE_COLUMN,
    VOLUME_COLUMN,
)


STOCKS_TABLE = "stocks"
STOCKS_NEXT_TABLE = "stocks_next"
STOCKS_COLUMNS = {
    "Ticker": "TEXT",
    "Company": "TEXT",
    "Sector": "TEXT",
    MARKET_CAP_COLUMN: "REAL",
    "Market Cap Score": "REAL",
    "Forward P/E": "REAL",
    "Forward P/E Score": "REAL",
    "PEG": "REAL",
    "PEG Score": "REAL",
    "P/S": "REAL",
    "P/S Score": "REAL",
    "P/FCF": "REAL",
    "P/FCF Score": "REAL",
    "EPS Past 5Y": "REAL",
    "EPS Past 5Y Score": "REAL",
    "Sales Past 5Y": "REAL",
    "Sales Past 5Y Score": "REAL",
    "EPS Quarter Over Quarter": "REAL",
    "Sales Quarter Over Quarter": "REAL",
    "ROE": "REAL",
    "ROE Score": "REAL",
    "ROIC": "REAL",
    "ROIC Score": "REAL",
    "Profit Margin": "REAL",
    "Profit Margin Score": "REAL",
    "Gross Margin": "REAL",
    "Operating Margin": "REAL",
    "Debt/Equity": "REAL",
    "Debt/Equity Score": "REAL",
    "Short Interest": "REAL",
    "200-Day Simple Moving Average": "REAL",
    "52W High": "REAL",
    "Target Price": "REAL",
    TARGET_PRICE_UPSIDE_COLUMN: "REAL",
    POTENTIAL_STOCK_COLUMN: "INTEGER",
    "Price": "REAL",
    CHANGE_PERCENT_COLUMN: "REAL",
    VOLUME_COLUMN: "REAL",
    "Quote Price": "REAL",
    "Quote Change": "REAL",
    "Quote Change Percent": "REAL",
    "Quote Volume": "REAL",
    FUNDAMENTAL_SCORE_COLUMN: "REAL",
    "Long Term Score": "REAL",
    "Mid Term Score": "REAL",
    "Short Term Score": "REAL",
    TECHNICAL_SCORE_COLUMN: "REAL",
    "EMA200Distance": "REAL",
    "ROC125": "REAL",
    "EMA50Distance": "REAL",
    "ROC20": "REAL",
    "PPO Slope3": "REAL",
    "RSI14": "REAL",
    TOTAL_SCORE_COLUMN: "REAL",
}
STOCKS_INDEX_COLUMNS = {
    "sector": "Sector",
    "market_cap": MARKET_CAP_COLUMN,
    "ticker": "Ticker",
    "fundamental_score": FUNDAMENTAL_SCORE_COLUMN,
    "technical_score": TECHNICAL_SCORE_COLUMN,
    "total_score": TOTAL_SCORE_COLUMN,
    "change_percent": CHANGE_PERCENT_COLUMN,
    "volume": VOLUME_COLUMN,
    "target_price_upside": TARGET_PRICE_UPSIDE_COLUMN,
    "potential_stock": POTENTIAL_STOCK_COLUMN,
}
STOCKS_SCREENER_SORT_INDEX_COLUMNS = {
    "market_cap": MARKET_CAP_COLUMN,
    "fundamental_score": FUNDAMENTAL_SCORE_COLUMN,
    "technical_score": TECHNICAL_SCORE_COLUMN,
    "total_score": TOTAL_SCORE_COLUMN,
    "change_percent": CHANGE_PERCENT_COLUMN,
    "volume": VOLUME_COLUMN,
    "target_price_upside": TARGET_PRICE_UPSIDE_COLUMN,
}


def quote_identifier(value: str) -> str:
    return f'"{value}"'


def stocks_select_columns_sql() -> str:
    return ", ".join(quote_identifier(column) for column in STOCKS_COLUMNS)


class StockTableSchema:
    def create_table_sql(self, table_name: str) -> str:
        columns_sql = ", ".join(
            f'{quote_identifier(column)} {column_type}'
            for column, column_type in STOCKS_COLUMNS.items()
        )
        return f'CREATE TABLE IF NOT EXISTS "{table_name}" ({columns_sql})'

    def ensure_table(self, connection, table_name: str) -> None:
        connection.execute(self.create_table_sql(table_name))
        self.add_missing_columns(connection, table_name)
        self.ensure_indexes(connection, table_name)
        connection.commit()

    def ensure_indexes(self, connection, table_name: str) -> None:
        for index_key, column in STOCKS_INDEX_COLUMNS.items():
            connection.execute(
                f'CREATE INDEX IF NOT EXISTS "{table_name}_{index_key}_idx" '
                f'ON "{table_name}" ({quote_identifier(column)})'
            )
        for index_key, column in STOCKS_SCREENER_SORT_INDEX_COLUMNS.items():
            connection.execute(
                f'CREATE INDEX IF NOT EXISTS "{table_name}_screener_{index_key}_desc_idx" '
                f'ON "{table_name}" ({quote_identifier(column)} DESC) '
                f'WHERE {quote_identifier(TOTAL_SCORE_COLUMN)} IS NOT NULL '
                f'AND {quote_identifier(VOLUME_COLUMN)} >= {MIN_VOLUME}'
            )

    def add_missing_columns(self, connection, table_name: str) -> None:
        existing_columns = self.fetch_table_columns(connection, table_name)
        for column, column_type in STOCKS_COLUMNS.items():
            if column not in existing_columns:
                connection.execute(
                    f'ALTER TABLE "{table_name}" ADD COLUMN '
                    f'{quote_identifier(column)} {column_type}'
                )

    def fetch_table_columns(self, connection, table_name: str) -> set[str]:
        cursor = connection.execute(f'PRAGMA table_info("{table_name}")')
        return {row[1] for row in cursor.fetchall()}

    def normalize_columns(self, data: pd.DataFrame) -> pd.DataFrame:
        normalized_data = data.copy()
        for column in STOCKS_COLUMNS:
            if column not in normalized_data.columns:
                normalized_data[column] = pd.NA
        normalized_data[POTENTIAL_STOCK_COLUMN] = (
            normalized_data[POTENTIAL_STOCK_COLUMN].fillna(False).astype(bool)
        )
        return normalized_data.loc[:, list(STOCKS_COLUMNS.keys())]
