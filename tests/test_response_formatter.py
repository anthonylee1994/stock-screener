import pandas as pd
import pytest

from stock_screener.services.api.screener_response_formatter import (
    format_record,
    format_response,
)


def test_format_response_maps_records_and_pagination_metadata():
    data = pd.DataFrame(
        [
            {
                "Ticker": "NVDA",
                "Company": "NVIDIA Corp",
                "Sector": "Technology",
                "Market Cap": 5_000_000_000_000.0,
                "Price": 110.0,
                "Change": 10.0,
                "Volume": 2_000_000,
                "Total Score": 88.8,
                "Potential Stock": 1,
                "Fundamental Score": 90.0,
                "Technical Score": 87.0,
            }
        ]
    )

    response = format_response(
        data,
        total_count=3,
        limit=1,
        offset=1,
        has_more=True,
    )

    assert response["count"] == 3
    assert response["limit"] == 1
    assert response["offset"] == 1
    assert response["has_more"] is True
    assert response["next_offset"] == 2
    assert response["data"][0]["ticker"] == "NVDA"
    assert response["data"][0]["change"] == pytest.approx(10.0)
    assert response["data"][0]["potential_stock"] is True
    assert response["data"][0]["fundamental"]["potential_stock"] == 1
    assert response["data"][0]["fundamental"]["fundamental_score"] == 90.0
    assert response["data"][0]["technical"]["technical_score"] == 87.0


def test_format_record_calculates_change_and_falls_back_to_quote_fields():
    row = pd.Series(
        {
            "Ticker": "MSFT",
            "Quote Price": 105.0,
            "Quote Change": 4.0,
            "Quote Change Percent": 2.5,
            "Price": 110.0,
            "Change": 10.0,
        }
    )

    record = format_record(row)

    assert record["price"] == 110.0
    assert record["change"] == pytest.approx(10.0)
    assert record["change_percent"] == 10.0


def test_format_record_uses_quote_change_when_price_change_percent_is_missing():
    row = pd.Series(
        {
            "Ticker": "MSFT",
            "Quote Change": 4.0,
        }
    )

    record = format_record(row)

    assert record["change"] == 4.0


def test_format_record_returns_none_for_missing_values():
    row = pd.Series({"Ticker": "AAPL", "Price": pd.NA})

    record = format_record(row)

    assert record["price"] is None
    assert record["change"] is None
    assert record["total_score"] is None


def test_format_record_returns_none_for_missing_change_sources():
    row = pd.Series(
        {
            "Ticker": "AAPL",
            "Change": pd.NA,
            "Quote Change": pd.NA,
        }
    )

    record = format_record(row)

    assert record["change"] is None
