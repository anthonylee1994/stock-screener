import logging
import os
import sqlite3
import threading
import time
from pathlib import Path
from typing import Any

import pandas as pd
from dotenv import load_dotenv

from stock_screener.utils.screener_rules import TOTAL_SCORE_COLUMN
from stock_screener.utils.stock_schema import (
    STOCKS_COLUMNS,
    STOCKS_NEXT_TABLE,
    STOCKS_TABLE,
    StockTableSchema,
    quote_identifier,
)
from stock_screener.utils.stock_screener_query_builder import (
    SEARCH_COLUMNS,
    StockScreenerQueryBuilder,
)


load_dotenv()


def resolve_database_path() -> Path:
    return Path(os.getenv("SQLITE_DB_PATH", "./data/db.sqlite"))


DATABASE_PATH = resolve_database_path()
logger = logging.getLogger(__name__)
DB_LOCK = threading.Lock()
INSERT_BATCH_SIZE = 100


class SQLiteConnection:
    def __init__(self, connection):
        self.connection = connection
        self.closed = False

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        if exc_type:
            self.connection.rollback()
        self.close()
        return False

    def execute(self, sql: str, params: tuple[Any, ...] | list[Any] | None = None):
        cursor = self.connection.cursor()
        start_time = time.perf_counter()
        if params is None:
            cursor.execute(sql)
        else:
            cursor.execute(sql, params)
        self.log_sql("execute", sql, params, start_time)
        return cursor

    def executemany(self, sql: str, rows: list[tuple[Any, ...]]) -> None:
        start_time = time.perf_counter()
        self.connection.executemany(sql, rows)
        self.log_sql("executemany", sql, {"rows": len(rows)}, start_time)

    def commit(self) -> None:
        self.connection.commit()

    def close(self) -> None:
        if self.closed:
            return
        self.closed = True
        self.connection.close()

    def log_sql(
        self,
        operation: str,
        sql: str,
        params: Any,
        start_time: float,
    ) -> None:
        elapsed_ms = (time.perf_counter() - start_time) * 1000
        logger.info(
            "SQL %s elapsed_ms=%.2f sql=%s params=%s",
            operation,
            elapsed_ms,
            self.normalize_sql_for_log(sql),
            params,
        )

    def normalize_sql_for_log(self, sql: str) -> str:
        return " ".join(str(sql).split())


class StockDatabase:
    def __init__(
        self,
        database_path: Path = DATABASE_PATH,
        schema: StockTableSchema | None = None,
        query_builder: StockScreenerQueryBuilder | None = None,
    ):
        self.database_path = Path(database_path)
        self.schema = schema or StockTableSchema()
        self.query_builder = query_builder or StockScreenerQueryBuilder()

    @property
    def display_name(self) -> str:
        return str(self.database_path)

    @property
    def placeholder(self) -> str:
        return self.query_builder.placeholder

    def initialize(self) -> None:
        with self.connect() as connection:
            self.schema.ensure_table(connection, STOCKS_TABLE)
        logger.info("股票資料庫已準備好 db=%s", self.display_name)

    def connect(self):
        start_time = time.perf_counter()
        self.database_path.parent.mkdir(parents=True, exist_ok=True)
        connection = sqlite3.connect(
            self.database_path, check_same_thread=False)
        elapsed_ms = (time.perf_counter() - start_time) * 1000
        logger.info("SQLite connect elapsed_ms=%.2f", elapsed_ms)
        return SQLiteConnection(connection)

    def read_dataframe(
        self,
        connection: SQLiteConnection,
        query: str,
        params: list[Any] | None = None,
    ) -> pd.DataFrame:
        logger.info("stocks table SQL query=%s params=%s", query, params or [])
        cursor = connection.execute(query, params or [])
        rows = cursor.fetchall()
        columns = [column[0] for column in cursor.description]
        return pd.DataFrame(rows, columns=columns)

    def has_stocks(self) -> bool:
        with self.connect() as connection:
            result = connection.execute(
                f'SELECT EXISTS(SELECT 1 FROM "{STOCKS_TABLE}" LIMIT 1)'
            ).fetchone()
        return bool(result and result[0])

    def read_stocks(self) -> pd.DataFrame:
        with self.connect() as connection:
            return self.read_dataframe(connection, f'SELECT * FROM "{STOCKS_TABLE}"')

    def read_screener_stocks_with_count(
        self,
        limit: int,
        order: str,
        ascend: bool,
        sector: str,
        market_cap: str,
        search: str = "",
        tickers: list[str] | None = None,
        offset: int = 0,
        potential_stock: bool = False,
    ) -> tuple[pd.DataFrame, int]:
        with self.connect() as connection:
            if tickers:
                data_query, data_params = self.query_builder.build_ticker_screener_query(
                    tickers=tickers,
                    order=order,
                    ascend=ascend,
                    limit=limit,
                    offset=offset,
                )
                count_query, count_params = (
                    self.query_builder.build_ticker_screener_count_query(
                        tickers)
                )
                data = self.read_dataframe(connection, data_query, data_params)
                total_count = self.read_count(
                    connection, count_query, count_params)
            else:
                data, total_count = self.read_search_screener_stocks_with_count(
                    connection=connection,
                    sector=sector,
                    market_cap=market_cap,
                    search=search,
                    order=order,
                    ascend=ascend,
                    limit=limit,
                    offset=offset,
                    potential_stock=potential_stock,
                )

        logger.info(
            "stocks table SQL screener rows=%s total=%s limit=%s offset=%s",
            len(data),
            total_count,
            limit,
            offset,
        )
        return data, total_count

    def read_search_screener_stocks_with_count(
        self,
        connection: SQLiteConnection,
        sector: str,
        market_cap: str,
        search: str,
        order: str,
        ascend: bool,
        limit: int,
        offset: int,
        potential_stock: bool = False,
    ) -> tuple[pd.DataFrame, int]:
        search_column, total_count = self.resolve_search_column_and_count(
            connection=connection,
            sector=sector,
            market_cap=market_cap,
            search=search,
            potential_stock=potential_stock,
        )
        query, params = self.query_builder.build_screener_query(
            sector=sector,
            market_cap=market_cap,
            search=search,
            order=order,
            ascend=ascend,
            limit=limit,
            offset=offset,
            potential_stock=potential_stock,
            search_column=search_column,
        )
        return self.read_dataframe(connection, query, params), total_count

    def resolve_search_column_and_count(
        self,
        connection: SQLiteConnection,
        sector: str,
        market_cap: str,
        search: str,
        potential_stock: bool = False,
    ) -> tuple[str | None, int]:
        normalized_search = str(search).strip()
        if not normalized_search:
            query, params = self.query_builder.build_screener_count_query(
                sector=sector,
                market_cap=market_cap,
                search="",
                potential_stock=potential_stock,
            )
            return None, self.read_count(connection, query, params)

        for search_column in SEARCH_COLUMNS:
            query, params = self.query_builder.build_screener_count_query(
                sector=sector,
                market_cap=market_cap,
                search=normalized_search,
                potential_stock=potential_stock,
                search_column=search_column,
            )
            total_count = self.read_count(connection, query, params)
            if total_count > 0 or search_column == SEARCH_COLUMNS[-1]:
                return search_column, total_count

        return SEARCH_COLUMNS[-1], 0

    def read_count(
        self,
        connection: SQLiteConnection,
        query: str,
        params: list[Any] | None = None,
    ) -> int:
        row = connection.execute(query, params or []).fetchone()
        if not row:
            return 0
        return int(row[0] or 0)

    def replace_stocks(self, data: pd.DataFrame) -> None:
        if data.empty:
            logger.warning("股票資料為空；唔會覆蓋 stocks table")
            return

        normalized_data = self.schema.normalize_columns(data)
        with DB_LOCK:
            with self.connect() as connection:
                connection.execute(
                    f'DROP TABLE IF EXISTS "{STOCKS_NEXT_TABLE}"')
                self.insert_stocks(
                    connection, STOCKS_NEXT_TABLE, normalized_data)
                connection.commit()
                connection.execute(f'DROP TABLE IF EXISTS "{STOCKS_TABLE}"')
                connection.execute(
                    f'ALTER TABLE "{STOCKS_NEXT_TABLE}" RENAME TO "{STOCKS_TABLE}"'
                )
                self.schema.ensure_indexes(connection, STOCKS_TABLE)
                connection.commit()

        logger.info("stocks table 已更新 rows=%s", len(normalized_data))

    def replace_scored_stocks(self, data: pd.DataFrame) -> pd.DataFrame:
        filtered_data = self.filter_scored_stocks(data)
        removed_count = len(data) - len(filtered_data)
        if removed_count:
            logger.info("已過濾冇 Total Score stocks rows=%s", removed_count)
        self.replace_stocks(filtered_data)
        return filtered_data

    def insert_stocks(
        self,
        connection,
        table_name: str,
        data: pd.DataFrame,
    ) -> None:
        connection.execute(self.schema.create_table_sql(table_name))
        quoted_columns = [quote_identifier(column)
                          for column in STOCKS_COLUMNS]
        placeholders = ", ".join(self.placeholder for _ in STOCKS_COLUMNS)
        insert_sql = (
            f'INSERT INTO "{table_name}" ({", ".join(quoted_columns)}) '
            f"VALUES ({placeholders})"
        )
        rows = [
            tuple(self.clean_database_value(value) for value in row)
            for row in data.itertuples(index=False, name=None)
        ]
        for batch_number, start in enumerate(
            range(0, len(rows), INSERT_BATCH_SIZE),
            start=1,
        ):
            batch = rows[start: start + INSERT_BATCH_SIZE]
            connection.executemany(insert_sql, batch)
            logger.info(
                "stocks insert batch table=%s batch=%s rows=%s inserted=%s total=%s",
                table_name,
                batch_number,
                len(batch),
                start + len(batch),
                len(rows),
            )

    def clean_database_value(self, value: Any) -> Any:
        if pd.isna(value):
            return None
        return value

    def filter_scored_stocks(self, data: pd.DataFrame) -> pd.DataFrame:
        if data.empty or TOTAL_SCORE_COLUMN not in data.columns:
            return data
        return data[data[TOTAL_SCORE_COLUMN].notna()].reset_index(drop=True)


stock_database = StockDatabase()
