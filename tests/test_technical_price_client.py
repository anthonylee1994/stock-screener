import pandas as pd

from stock_screener.services.technical import technical_price_client as price_client_module
from stock_screener.services.technical.technical_price_client import (
    DEFAULT_INTERVAL,
    DEFAULT_PERIOD,
    DOWNLOAD_RETRY_CHUNK_SIZE,
    TechnicalPriceClient,
)


def make_price_frame(ticker: str, close_values: list[float]) -> pd.DataFrame:
    columns = pd.MultiIndex.from_product([["Close", "Volume"], [ticker]])
    return pd.DataFrame(
        [
            [close, 1000 + index]
            for index, close in enumerate(close_values)
        ],
        columns=columns,
    )


class RetryingTechnicalPriceClient(TechnicalPriceClient):
    def __init__(self):
        self.calls = []

    def download_tickers(self, tickers: list[str]) -> pd.DataFrame:
        self.calls.append(tickers)
        if tickers == ["AAPL", "MSFT"]:
            return make_price_frame("AAPL", [1.0, 2.0])
        return make_price_frame("MSFT", [3.0, 4.0])


def test_download_price_data_retries_failed_tickers_and_merges_result():
    client = RetryingTechnicalPriceClient()

    result = client.download_price_data(["AAPL", "MSFT"])

    assert client.calls == [["AAPL", "MSFT"], ["MSFT"]]
    assert client.find_failed_tickers(result, ["AAPL", "MSFT"]) == []
    assert result["Close"]["AAPL"].tolist() == [1.0, 2.0]
    assert result["Close"]["MSFT"].tolist() == [3.0, 4.0]


class AlwaysFailingTechnicalPriceClient(TechnicalPriceClient):
    def download_tickers(self, tickers: list[str]) -> pd.DataFrame:
        return pd.DataFrame()


def test_download_price_data_returns_empty_frame_when_retries_still_fail():
    client = AlwaysFailingTechnicalPriceClient()

    result = client.download_price_data(["AAPL"])

    assert result.empty


def test_download_tickers_calls_yfinance_and_normalizes_result(monkeypatch):
    calls = []
    downloaded = pd.DataFrame({"Close": [1.0], "Volume": [100]})

    def fake_download(**kwargs):
        calls.append(kwargs)
        return downloaded

    monkeypatch.setattr(price_client_module.yf, "download", fake_download)
    client = TechnicalPriceClient()

    result = client.download_tickers(["AAPL"])

    assert calls == [
        {
            "tickers": ["AAPL"],
            "period": DEFAULT_PERIOD,
            "interval": DEFAULT_INTERVAL,
            "auto_adjust": True,
            "progress": True,
            "group_by": "column",
            "threads": True,
        }
    ]
    assert isinstance(result.columns, pd.MultiIndex)
    assert result["Close"]["AAPL"].tolist() == [1.0]


def test_download_tickers_returns_empty_frame_on_yfinance_error(monkeypatch):
    def fake_download(**kwargs):
        raise RuntimeError("network failed")

    monkeypatch.setattr(price_client_module.yf, "download", fake_download)
    client = TechnicalPriceClient()

    result = client.download_tickers(["AAPL"])

    assert result.empty


class ChunkingTechnicalPriceClient(TechnicalPriceClient):
    def __init__(self, empty: bool = False):
        self.empty = empty
        self.calls = []

    def download_tickers(self, tickers: list[str]) -> pd.DataFrame:
        self.calls.append(tickers)
        if self.empty:
            return pd.DataFrame()
        return make_price_frame(tickers[0], [1.0])


def test_download_tickers_in_chunks_merges_non_empty_chunks():
    client = ChunkingTechnicalPriceClient()
    tickers = [f"TICKER{index}" for index in range(
        DOWNLOAD_RETRY_CHUNK_SIZE + 1)]

    result = client.download_tickers_in_chunks(tickers)

    assert client.calls == [tickers[:DOWNLOAD_RETRY_CHUNK_SIZE], tickers[25:]]
    assert result["Close"]["TICKER0"].tolist() == [1.0]
    assert result["Close"]["TICKER25"].tolist() == [1.0]


def test_download_tickers_in_chunks_returns_empty_frame_when_all_chunks_empty():
    client = ChunkingTechnicalPriceClient(empty=True)

    result = client.download_tickers_in_chunks(["AAPL"])

    assert result.empty


def test_normalize_price_data_handles_empty_multi_single_and_plain_multi_ticker():
    client = TechnicalPriceClient()
    multi_index_data = make_price_frame("AAPL", [1.0])
    single_ticker_data = pd.DataFrame({"Close": [1.0]})
    multi_ticker_plain_data = pd.DataFrame({"Close": [1.0]})

    assert client.normalize_price_data(pd.DataFrame(), ["AAPL"]).empty
    assert client.normalize_price_data(
        multi_index_data, ["AAPL"]).equals(multi_index_data)
    assert isinstance(
        client.normalize_price_data(single_ticker_data, ["AAPL"]).columns,
        pd.MultiIndex,
    )
    assert client.normalize_price_data(
        multi_ticker_plain_data,
        ["AAPL", "MSFT"],
    ).equals(multi_ticker_plain_data)


def test_merge_price_data_ignores_empty_frames_and_keeps_latest_duplicate_columns():
    client = TechnicalPriceClient()
    older = make_price_frame("AAPL", [1.0])
    newer = make_price_frame("AAPL", [2.0])

    assert client.merge_price_data(pd.DataFrame()).empty
    result = client.merge_price_data(older, pd.DataFrame(), newer)

    assert result["Close"]["AAPL"].tolist() == [2.0]


def test_extract_field_handles_empty_missing_multiindex_single_ticker_and_fallback():
    client = TechnicalPriceClient()
    multi_index_data = make_price_frame("AAPL", [1.0])
    single_ticker_data = pd.DataFrame({"Close": [1.0]})

    assert client.extract_field(pd.DataFrame(), ["AAPL"], "Close").empty
    assert client.extract_field(multi_index_data, ["AAPL"], "Open").empty
    assert client.extract_field(multi_index_data, ["AAPL"], "Close")[
        "AAPL"].tolist() == [1.0]
    assert client.extract_field(single_ticker_data, ["AAPL"], "Close")[
        "AAPL"].tolist() == [1.0]
    assert client.extract_field(
        single_ticker_data, ["AAPL", "MSFT"], "Close").empty
