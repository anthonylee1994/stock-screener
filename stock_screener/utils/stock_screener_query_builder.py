from typing import Any

from stock_screener.utils.screener_rules import (
    MARKET_CAP_COLUMN,
    MARKET_CAP_RANGES,
    MIN_VOLUME,
    SEARCH_COLUMNS,
    TOTAL_SCORE_COLUMN,
    VOLUME_COLUMN,
    normalize_sort_value,
)
from stock_screener.utils.stock_schema import (
    STOCKS_COLUMNS,
    STOCKS_TABLE,
    quote_identifier,
    stocks_select_columns_sql,
)
from stock_screener.utils.ticker_normalizer import normalize_tickers


class ScreenerFilters:
    def __init__(self, placeholder: str):
        self.placeholder = placeholder
        self.where_sql = [
            f"{quote_identifier(TOTAL_SCORE_COLUMN)} IS NOT NULL",
            f"{quote_identifier(VOLUME_COLUMN)} >= {MIN_VOLUME}",
        ]
        self.params: list[Any] = []

    def add_sector(self, sector: str) -> None:
        if sector == "All":
            return
        self.where_sql.append(f'"Sector" = {self.placeholder}')
        self.params.append(sector)

    def add_market_cap(self, market_cap: str) -> None:
        min_cap, max_cap = MARKET_CAP_RANGES.get(market_cap, (None, None))
        if min_cap is not None:
            self.where_sql.append(
                f"{quote_identifier(MARKET_CAP_COLUMN)} >= {self.placeholder}"
            )
            self.params.append(min_cap)
        if max_cap is not None:
            self.where_sql.append(
                f"{quote_identifier(MARKET_CAP_COLUMN)} < {self.placeholder}"
            )
            self.params.append(max_cap)

    def add_search(
        self,
        search_column: str | None,
        like_value: str,
    ) -> None:
        if search_column is None:
            conditions = [
                search_condition_sql(column, self.placeholder)
                for column in SEARCH_COLUMNS
            ]
            self.where_sql.append(f"({' OR '.join(conditions)})")
            self.params.extend([like_value for _ in SEARCH_COLUMNS])
            return

        self.where_sql.append(search_condition_sql(search_column, self.placeholder))
        self.params.append(like_value)


class StockScreenerQueryBuilder:
    def __init__(self, placeholder: str = "?"):
        self.placeholder = placeholder

    def build_screener_query(
        self,
        sector: str,
        market_cap: str,
        search: str,
        order: str,
        ascend: bool,
        limit: int,
        offset: int,
        search_column: str | None = None,
    ) -> tuple[str, list[Any]]:
        filters = self.build_screener_filters(
            sector=sector,
            market_cap=market_cap,
            search=search,
            search_column=search_column,
        )
        return self.build_select_query(
            where_sql=filters.where_sql,
            params=filters.params,
            order=order,
            ascend=ascend,
            limit=limit,
            offset=offset,
        )

    def build_screener_count_query(
        self,
        sector: str,
        market_cap: str,
        search: str,
        search_column: str | None = None,
    ) -> tuple[str, list[Any]]:
        filters = self.build_screener_filters(
            sector=sector,
            market_cap=market_cap,
            search=search,
            search_column=search_column,
        )
        return self.build_count_query(
            where_sql=filters.where_sql,
            params=filters.params,
        )

    def build_screener_filters(
        self,
        sector: str,
        market_cap: str,
        search: str,
        search_column: str | None = None,
    ) -> ScreenerFilters:
        filters = ScreenerFilters(self.placeholder)
        filters.add_sector(sector)
        filters.add_market_cap(market_cap)

        normalized_search = str(search).strip()
        if normalized_search:
            filters.add_search(
                search_column=search_column,
                like_value=self.like_contains_value(normalized_search),
            )

        return filters

    def build_ticker_screener_query(
        self,
        tickers: list[str],
        order: str,
        ascend: bool,
        limit: int,
        offset: int,
    ) -> tuple[str, list[Any]]:
        normalized_tickers = normalize_tickers(tickers)
        if not normalized_tickers:
            return self.empty_select_query()

        where_sql, params = self.build_ticker_filter(normalized_tickers)
        return self.build_select_query(
            where_sql=where_sql,
            params=params,
            order=order,
            ascend=ascend,
            limit=limit,
            offset=offset,
        )

    def build_ticker_screener_count_query(
        self,
        tickers: list[str],
    ) -> tuple[str, list[Any]]:
        normalized_tickers = normalize_tickers(tickers)
        if not normalized_tickers:
            return self.empty_count_query()

        where_sql, params = self.build_ticker_filter(normalized_tickers)
        return self.build_count_query(where_sql=where_sql, params=params)

    def build_ticker_filter(self, tickers: list[str]) -> tuple[list[str], list[Any]]:
        placeholders = ", ".join(self.placeholder for _ in tickers)
        where_sql = [f'UPPER("Ticker") IN ({placeholders})']
        params: list[Any] = tickers
        return where_sql, params

    def build_select_query(
        self,
        where_sql: list[str],
        params: list[Any],
        order: str,
        ascend: bool,
        limit: int,
        offset: int,
    ) -> tuple[str, list[Any]]:
        order_column = self.normalize_sort_column(order)
        direction = "ASC" if ascend else "DESC"
        query = (
            f'SELECT {stocks_select_columns_sql()} FROM "{STOCKS_TABLE}" '
            f"WHERE {' AND '.join(where_sql)} "
            f'ORDER BY "{order_column}" IS NULL, "{order_column}" {direction}, '
            f'"Ticker" ASC '
            f"LIMIT {self.placeholder} OFFSET {self.placeholder}"
        )
        return query, [*params, max(0, int(limit)), max(0, int(offset))]

    def build_count_query(
        self,
        where_sql: list[str],
        params: list[Any],
    ) -> tuple[str, list[Any]]:
        query = (
            f'SELECT COUNT(*) AS count FROM "{STOCKS_TABLE}" '
            f"WHERE {' AND '.join(where_sql)}"
        )
        return query, params

    def empty_select_query(self) -> tuple[str, list[Any]]:
        return f'SELECT {stocks_select_columns_sql()} FROM "{STOCKS_TABLE}" WHERE 0', []

    def empty_count_query(self) -> tuple[str, list[Any]]:
        return "SELECT 0 AS count", []

    def normalize_sort_column(self, order: str) -> str:
        column = normalize_sort_value(order)
        if column == "Quote Volume":
            column = VOLUME_COLUMN
        if column not in STOCKS_COLUMNS:
            column = str(column).removesuffix(".")
        if column not in STOCKS_COLUMNS:
            return MARKET_CAP_COLUMN
        return column

    def like_contains_value(self, value: str) -> str:
        return f"%{self.escape_like_value(str(value).lower())}%"

    def escape_like_value(self, value: str) -> str:
        return value.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_")


def search_condition_sql(column: str, placeholder: str) -> str:
    return f"LOWER(\"{column}\") LIKE {placeholder} ESCAPE '\\'"
