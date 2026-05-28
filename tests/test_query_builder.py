from stock_screener.utils.stock_screener_query_builder import (
    StockScreenerQueryBuilder,
    search_condition_sql,
)


def test_build_screener_query_includes_filters_search_sort_and_pagination():
    builder = StockScreenerQueryBuilder()

    query, params = builder.build_screener_query(
        sector="Technology",
        market_cap="Large",
        search="a_p%",
        order="volume",
        ascend=True,
        limit=25,
        offset=50,
    )

    assert '"Sector" = ?' in query
    assert '"Market Cap" >= ?' in query
    assert '"Market Cap" < ?' in query
    assert '(LOWER("Ticker") LIKE ? ESCAPE' in query
    assert 'LOWER("Company") LIKE ? ESCAPE' in query
    assert 'ORDER BY "Volume" IS NULL, "Volume" ASC, "Ticker" ASC' in query
    assert params == [
        "Technology",
        10_000_000_000,
        200_000_000_000,
        "%a\\_p\\%%",
        "%a\\_p\\%%",
        25,
        50,
    ]


def test_build_ticker_screener_query_normalizes_and_ignores_blank_tickers():
    builder = StockScreenerQueryBuilder(placeholder="%s")

    query, params = builder.build_ticker_screener_query(
        tickers=[" aapl ", "", "msft"],
        order="total_score",
        ascend=False,
        limit=-5,
        offset=-1,
    )

    assert 'UPPER("Ticker") IN (%s, %s)' in query
    assert 'ORDER BY "Total Score" IS NULL, "Total Score" DESC, "Ticker" ASC' in query
    assert params == ["AAPL", "MSFT", 0, 0]


def test_build_ticker_screener_query_returns_empty_query_for_no_tickers():
    builder = StockScreenerQueryBuilder()

    query, params = builder.build_ticker_screener_query(
        tickers=["", " "],
        order="market_cap",
        ascend=False,
        limit=10,
        offset=0,
    )

    assert query.endswith("WHERE 0")
    assert params == []


def test_normalize_sort_column_falls_back_to_market_cap():
    builder = StockScreenerQueryBuilder()

    assert builder.normalize_sort_column("Quote Volume") == "Volume"
    assert builder.normalize_sort_column("unknown") == "Market Cap"
    assert builder.normalize_sort_column("Market Cap.") == "Market Cap"


def test_search_condition_sql_uses_lowercase_like_with_escape():
    assert search_condition_sql("Ticker", "?") == 'LOWER("Ticker") LIKE ? ESCAPE \'\\\''
