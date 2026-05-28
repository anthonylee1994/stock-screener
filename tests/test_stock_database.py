import sqlite3

import pandas as pd
import pytest

from stock_screener.utils import stock_database as stock_database_module
from stock_screener.utils.screener_rules import (
    MARKET_CAP_COLUMN,
    TOTAL_SCORE_COLUMN,
    VOLUME_COLUMN,
)
from stock_screener.utils.stock_database import SQLiteConnection, StockDatabase
from stock_screener.utils.stock_schema import STOCKS_COLUMNS, STOCKS_TABLE, StockTableSchema


def test_sqlite_connection_executes_commits_rolls_back_and_closes_once(tmp_path):
    database_path = tmp_path / "connection.sqlite"
    raw_connection = sqlite3.connect(database_path)
    connection = SQLiteConnection(raw_connection)

    connection.execute("CREATE TABLE items (name TEXT)")
    connection.executemany(
        "INSERT INTO items (name) VALUES (?)", [("before",)])
    connection.commit()

    with pytest.raises(RuntimeError):
        with connection:
            connection.execute(
                "INSERT INTO items (name) VALUES (?)", ("rolled back",))
            raise RuntimeError("rollback")

    connection.close()
    assert connection.closed is True
    assert connection.normalize_sql_for_log(
        " SELECT   *\nFROM items ") == "SELECT * FROM items"

    check_connection = sqlite3.connect(database_path)
    rows = check_connection.execute("SELECT name FROM items").fetchall()
    check_connection.close()

    assert rows == [("before",)]


def make_stock_frame() -> pd.DataFrame:
    return pd.DataFrame(
        [
            {
                "Ticker": "AAPL",
                "Company": "Apple Inc",
                "Sector": "Technology",
                MARKET_CAP_COLUMN: 3_000_000_000_000,
                VOLUME_COLUMN: 10_000_000,
                TOTAL_SCORE_COLUMN: 90.0,
            },
            {
                "Ticker": "MSFT",
                "Company": "Microsoft Corp",
                "Sector": "Technology",
                MARKET_CAP_COLUMN: 2_500_000_000_000,
                VOLUME_COLUMN: 9_000_000,
                TOTAL_SCORE_COLUMN: pd.NA,
            },
            {
                "Ticker": "JPM",
                "Company": "JPMorgan Chase",
                "Sector": "Financial",
                MARKET_CAP_COLUMN: 500_000_000_000,
                VOLUME_COLUMN: 8_000_000,
                TOTAL_SCORE_COLUMN: 75.0,
            },
        ]
    )


def test_stock_database_replaces_filters_reads_and_counts_stocks(tmp_path):
    database = StockDatabase(tmp_path / "stocks.sqlite")

    assert database.display_name.endswith("stocks.sqlite")
    assert database.placeholder == "?"
    database.initialize()
    assert database.has_stocks() is False

    database.replace_stocks(pd.DataFrame())
    filtered_data = database.replace_scored_stocks(make_stock_frame())

    assert filtered_data["Ticker"].tolist() == ["AAPL", "JPM"]
    assert database.has_stocks() is True
    assert database.clean_database_value(pd.NA) is None
    assert database.clean_database_value("AAPL") == "AAPL"
    assert database.filter_scored_stocks(pd.DataFrame()).empty
    assert database.filter_scored_stocks(pd.DataFrame({"Ticker": ["AAPL"]}))["Ticker"].tolist() == [
        "AAPL"
    ]

    stored_data = database.read_stocks()
    assert stored_data["Ticker"].tolist() == ["AAPL", "JPM"]
    assert set(STOCKS_COLUMNS).issubset(stored_data.columns)

    ticker_data, ticker_count = database.read_screener_stocks_with_count(
        limit=10,
        order=TOTAL_SCORE_COLUMN,
        ascend=False,
        sector="All",
        market_cap="+Mid",
        tickers=["jpm", "aapl"],
    )
    assert ticker_count == 2
    assert ticker_data["Ticker"].tolist() == ["AAPL", "JPM"]

    search_data, search_count = database.read_screener_stocks_with_count(
        limit=10,
        order="unknown",
        ascend=True,
        sector="Technology",
        market_cap="Mega",
        search="Apple",
    )
    assert search_count == 1
    assert search_data["Ticker"].tolist() == ["AAPL"]

    with database.connect() as connection:
        assert database.resolve_search_column_and_count(
            connection=connection,
            sector="All",
            market_cap="Unknown",
            search="",
        ) == (None, 2)
        assert database.resolve_search_column_and_count(
            connection=connection,
            sector="All",
            market_cap="Unknown",
            search="not-found",
        ) == ("Company", 0)
        assert database.read_count(connection, "SELECT 1 WHERE 0") == 0


def test_resolve_search_column_falls_back_after_empty_search_column_iteration(
    tmp_path,
    monkeypatch,
):
    class EmptySearchColumns:
        def __iter__(self):
            return iter(())

        def __getitem__(self, index):
            assert index == -1
            return "Company"

    database = StockDatabase(tmp_path / "fallback.sqlite")
    database.initialize()
    monkeypatch.setattr(stock_database_module,
                        "SEARCH_COLUMNS", EmptySearchColumns())

    with database.connect() as connection:
        assert database.resolve_search_column_and_count(
            connection=connection,
            sector="All",
            market_cap="Unknown",
            search="anything",
        ) == ("Company", 0)


def test_stock_schema_adds_missing_columns_and_normalizes_data(tmp_path):
    database = StockDatabase(tmp_path / "schema.sqlite")
    schema = StockTableSchema()

    with database.connect() as connection:
        connection.execute(f'CREATE TABLE "{STOCKS_TABLE}" ("Ticker" TEXT)')
        connection.commit()

    with database.connect() as connection:
        schema.ensure_table(connection, STOCKS_TABLE)
        columns = schema.fetch_table_columns(connection, STOCKS_TABLE)

    normalized_data = schema.normalize_columns(
        pd.DataFrame([{"Ticker": "AAPL", "Extra": 1}]))

    assert "Company" in columns
    assert list(normalized_data.columns) == list(STOCKS_COLUMNS)
    assert normalized_data.loc[0, "Ticker"] == "AAPL"
    assert pd.isna(normalized_data.loc[0, "Company"])
