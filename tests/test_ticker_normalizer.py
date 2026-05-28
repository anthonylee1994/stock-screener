from stock_screener.utils.ticker_normalizer import normalize_tickers


def test_normalize_tickers_handles_none_strings_duplicates_and_iterables():
    assert normalize_tickers(None) == []
    assert normalize_tickers(" aapl, msft, AAPL ,, ") == ["AAPL", "MSFT"]
    assert normalize_tickers([" nvda ", "NVDA", None]) == ["NVDA", "NONE"]
