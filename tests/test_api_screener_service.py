import pandas as pd

from stock_screener.services.api import screener_service as screener_service_module
from stock_screener.services.api.screener_service import ScreenerService
from stock_screener.utils.screener_rules import TOTAL_SCORE_COLUMN


class FakeScreenerDatabase:
    def __init__(self, data: pd.DataFrame, total_count: int):
        self.data = data
        self.total_count = total_count
        self.calls = []

    def read_screener_stocks_with_count(self, **kwargs):
        self.calls.append(kwargs)
        return self.data, self.total_count


def test_screener_service_checks_api_token(monkeypatch):
    monkeypatch.setattr(screener_service_module, "API_TOKEN", "secret")
    service = ScreenerService(database=FakeScreenerDatabase(pd.DataFrame(), 0))

    assert service.is_authorized({"api_token": "secret"}) is True
    assert service.is_authorized({"api_token": "wrong"}) is False
    assert service.is_authorized({}) is False


def test_screener_service_requests_one_extra_row_and_formats_page():
    data = pd.DataFrame(
        [
            {"Ticker": "AAPL", "Company": "Apple", "Total Score": 90.0},
            {"Ticker": "MSFT", "Company": "Microsoft", "Total Score": 80.0},
            {"Ticker": "NVDA", "Company": "NVIDIA", "Total Score": 70.0},
        ]
    )
    database = FakeScreenerDatabase(data, total_count=5)
    service = ScreenerService(database=database)

    response = service.get_screener_response(
        {
            "sector": "Technology",
            "market_cap": "large",
            "search": "aapl",
            "tickers": "aapl,msft",
            "order": "total_score",
            "ascend": "true",
            "limit": 2,
            "offset": 4,
            "potential_stock": "true",
        }
    )

    assert database.calls == [
        {
            "limit": 3,
            "sector": "Technology",
            "market_cap": "Large",
            "search": "aapl",
            "tickers": ["AAPL", "MSFT"],
            "order": TOTAL_SCORE_COLUMN,
            "ascend": True,
            "offset": 4,
            "potential_stock": True,
        }
    ]
    assert response["count"] == 5
    assert response["limit"] == 2
    assert response["offset"] == 4
    assert response["has_more"] is True
    assert response["next_offset"] == 6
    assert [record["ticker"]
            for record in response["data"]] == ["AAPL", "MSFT"]
