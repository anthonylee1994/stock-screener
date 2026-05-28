import pandas as pd
import pytest

from stock_screener.services.integrated.integrated_screener_builder import (
    IntegratedScreenerBuilder,
)
from stock_screener.utils.screener_rules import (
    FUNDAMENTAL_SCORE_COLUMN,
    TECHNICAL_SCORE_COLUMN,
    TOTAL_SCORE_COLUMN,
)


class FakeIntegratedFundamentalService:
    def __init__(self, data: pd.DataFrame):
        self.data = data
        self.calls = []

    def run(self, limit: int):
        self.calls.append(limit)
        return self.data.copy()


class FakeIntegratedTechnicalService:
    def __init__(self, data: pd.DataFrame):
        self.data = data
        self.calls = []

    def run(self, tickers: list[str]):
        self.calls.append(tickers)
        return self.data.copy()


def test_integrated_screener_builder_merges_technical_data_and_adds_weighted_score():
    fundamental_data = pd.DataFrame(
        [
            {"Ticker": "AAPL", FUNDAMENTAL_SCORE_COLUMN: 90.0},
            {"Ticker": "MSFT", FUNDAMENTAL_SCORE_COLUMN: 70.0},
        ]
    )
    technical_data = pd.DataFrame(
        [
            {"Ticker": "AAPL", TECHNICAL_SCORE_COLUMN: 50.0},
            {"Ticker": "MSFT", TECHNICAL_SCORE_COLUMN: 100.0},
        ]
    )
    fundamental_service = FakeIntegratedFundamentalService(fundamental_data)
    technical_service = FakeIntegratedTechnicalService(technical_data)
    builder = IntegratedScreenerBuilder(
        fundamental_service=fundamental_service,
        technical_service=technical_service,
    )

    result = builder.build(limit=2)

    assert fundamental_service.calls == [2]
    assert technical_service.calls == [["AAPL", "MSFT"]]
    assert result["Ticker"].tolist() == ["AAPL", "MSFT"]
    assert result[TOTAL_SCORE_COLUMN].tolist() == pytest.approx([0.0, 100.0])


def test_integrated_screener_builder_returns_fundamental_copy_when_no_ticker_column():
    fundamental_data = pd.DataFrame([{FUNDAMENTAL_SCORE_COLUMN: 90.0}])
    fundamental_service = FakeIntegratedFundamentalService(fundamental_data)
    technical_service = FakeIntegratedTechnicalService(pd.DataFrame())
    builder = IntegratedScreenerBuilder(
        fundamental_service=fundamental_service,
        technical_service=technical_service,
    )

    result = builder.build(limit=2)

    assert technical_service.calls == []
    assert result.equals(fundamental_data)
    assert result is not fundamental_data


def test_integrated_screener_builder_adds_empty_total_score_when_technical_data_empty():
    fundamental_data = pd.DataFrame(
        [{"Ticker": "AAPL", FUNDAMENTAL_SCORE_COLUMN: 90.0}]
    )
    fundamental_service = FakeIntegratedFundamentalService(fundamental_data)
    technical_service = FakeIntegratedTechnicalService(pd.DataFrame())
    builder = IntegratedScreenerBuilder(
        fundamental_service=fundamental_service,
        technical_service=technical_service,
    )

    result = builder.build(limit=1)

    assert technical_service.calls == [["AAPL"]]
    assert pd.isna(result.loc[0, TOTAL_SCORE_COLUMN])


def test_integrated_screener_builder_add_total_score_returns_empty_data_unchanged():
    builder = IntegratedScreenerBuilder()
    data = pd.DataFrame()

    result = builder.add_total_score(data)

    assert result is data


def test_integrated_screener_builder_curves_total_score_between_zero_and_one_hundred():
    builder = IntegratedScreenerBuilder()
    data = pd.DataFrame(
        [
            {"Ticker": "LOW", FUNDAMENTAL_SCORE_COLUMN: 50.0,
                TECHNICAL_SCORE_COLUMN: 50.0},
            {"Ticker": "MID", FUNDAMENTAL_SCORE_COLUMN: 70.0,
                TECHNICAL_SCORE_COLUMN: 70.0},
            {
                "Ticker": "HIGH",
                FUNDAMENTAL_SCORE_COLUMN: 90.0,
                TECHNICAL_SCORE_COLUMN: 90.0,
            },
        ]
    )

    result = builder.add_total_score(data)

    assert result[TOTAL_SCORE_COLUMN].tolist() == pytest.approx([
        0.0, 50.0, 100.0])


def test_integrated_screener_builder_curves_single_valid_total_score_to_one_hundred():
    builder = IntegratedScreenerBuilder()
    data = pd.DataFrame(
        [
            {"Ticker": "VALID", FUNDAMENTAL_SCORE_COLUMN: 50.0,
                TECHNICAL_SCORE_COLUMN: 50.0},
            {"Ticker": "MISSING", FUNDAMENTAL_SCORE_COLUMN: 70.0,
                TECHNICAL_SCORE_COLUMN: pd.NA},
        ]
    )

    result = builder.add_total_score(data)

    assert result.loc[0, TOTAL_SCORE_COLUMN] == 100.0
    assert pd.isna(result.loc[1, TOTAL_SCORE_COLUMN])
