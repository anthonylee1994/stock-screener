from stock_screener.services.api.screener_request import ScreenerRequest
from stock_screener.utils.screener_rules import (
    MARKET_CAP_COLUMN,
    TOTAL_SCORE_COLUMN,
)


def test_screener_request_normalizes_api_values_and_bounds_numbers():
    request = ScreenerRequest(
        {
            "market_cap": "large",
            "order": "total_score",
            "ascend": "yes",
            "limit": "250",
            "offset": "-10",
            "tickers": " aapl, MSFT, aapl, , nvda ",
        }
    )

    assert request.market_cap == "Large"
    assert request.order == TOTAL_SCORE_COLUMN
    assert request.ascend is True
    assert request.limit == 100
    assert request.offset == 0
    assert request.tickers == ["AAPL", "MSFT", "NVDA"]


def test_screener_request_uses_defaults_for_invalid_values():
    request = ScreenerRequest({"limit": "bad", "offset": "bad"})

    assert request.market_cap == "+Large"
    assert request.order == TOTAL_SCORE_COLUMN
    assert request.ascend is False
    assert request.limit == 100
    assert request.offset == 0


def test_screener_request_maps_market_cap_sort_alias():
    request = ScreenerRequest({"order": "market_cap"})

    assert request.order == MARKET_CAP_COLUMN


def test_screener_request_parse_int_applies_minimum_without_maximum():
    request = ScreenerRequest({})

    assert request.parse_int("5", default=0, minimum=10) == 10


def test_screener_request_parse_bool_returns_default_for_none():
    request = ScreenerRequest({})

    assert request.parse_bool(None, default=True) is True


def test_screener_request_parse_bool_returns_bool_values_directly():
    request = ScreenerRequest({})

    assert request.parse_bool(True, default=False) is True
    assert request.parse_bool(False, default=True) is False
