import pandas as pd

from stock_screener.services.fundamental import fundamental_screener_client as client_module
from stock_screener.services.fundamental.fundamental_screener_client import (
    CUSTOM_COLUMNS,
    DEFAULT_ASCEND,
    DEFAULT_ORDER,
    FUNDAMENTAL_FILTERS,
    FundamentalScreenerClient,
)


class FakeCustom:
    instances = []

    def __init__(self, result):
        self.result = result
        self.filters = None
        self.view_calls = []
        FakeCustom.instances.append(self)

    def set_filter(self, filters_dict: dict):
        self.filters = filters_dict

    def screener_view(self, **kwargs):
        self.view_calls.append(kwargs)
        return self.result


def test_fundamental_screener_client_fetch_configures_finviz_custom(monkeypatch):
    result = pd.DataFrame([{"Ticker": "AAPL"}])

    def custom_factory():
        return FakeCustom(result)

    monkeypatch.setattr(client_module, "Custom", custom_factory)
    client = FundamentalScreenerClient()

    fetched = client.fetch(limit=50)

    custom = FakeCustom.instances[-1]
    assert fetched.equals(result)
    assert custom.filters == FUNDAMENTAL_FILTERS
    assert custom.filters is not FUNDAMENTAL_FILTERS
    assert custom.view_calls == [
        {
            "order": DEFAULT_ORDER,
            "limit": 50,
            "verbose": 1,
            "ascend": DEFAULT_ASCEND,
            "columns": CUSTOM_COLUMNS,
            "sleep_sec": 0.2,
        }
    ]


def test_fundamental_screener_client_fetch_returns_empty_frame_for_none(monkeypatch):
    def custom_factory():
        return FakeCustom(None)

    monkeypatch.setattr(client_module, "Custom", custom_factory)
    client = FundamentalScreenerClient()

    result = client.fetch(limit=10)

    assert result.empty


def test_fundamental_screener_client_requests_potential_stock_columns():
    assert 23 in CUSTOM_COLUMNS
    assert 30 in CUSTOM_COLUMNS
    assert 39 in CUSTOM_COLUMNS
    assert 40 in CUSTOM_COLUMNS
    assert 54 in CUSTOM_COLUMNS
    assert 57 in CUSTOM_COLUMNS
    assert 69 in CUSTOM_COLUMNS
