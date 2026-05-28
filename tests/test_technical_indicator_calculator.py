import pandas as pd
import pytest

from stock_screener.services.technical.technical_indicator_calculator import (
    TechnicalIndicatorCalculator,
)


class FakeTechnicalPriceClient:
    def extract_close_prices(self, price_data: pd.DataFrame, tickers: list[str]):
        return price_data.loc[:, tickers]

    def extract_field(
        self,
        price_data: pd.DataFrame,
        tickers: list[str],
        field: str,
    ):
        return price_data.loc[:, [f"{ticker} {field}" for ticker in tickers]].rename(
            columns={f"{ticker} {field}": ticker for ticker in tickers}
        )


def test_technical_indicator_calculator_calculates_rows_for_valid_tickers():
    calculator = TechnicalIndicatorCalculator(
        price_client=FakeTechnicalPriceClient())
    close = pd.Series(range(1, 203), dtype="float64")
    price_data = pd.DataFrame(
        {
            "AAPL": close,
            "AAPL Volume": pd.Series(range(1_000, 1_202), dtype="float64"),
        }
    )

    result = calculator.calculate_indicators(price_data, tickers=["AAPL"])

    row = result.iloc[0]
    assert row["Ticker"] == "AAPL"
    assert row["Quote Price"] == 202.0
    assert row["Quote Change"] == 1.0
    assert row["Quote Change Percent"] == pytest.approx(1 / 201)
    assert row["Quote Volume"] == 1201.0
    assert row["EMA200Distance"] > 0
    assert row["ROC125"] == pytest.approx((202 / 77) - 1)
    assert row["ROC20"] == pytest.approx((202 / 182) - 1)
    assert pd.notna(row["PPO Slope3"])
    assert row["RSI14"] == pytest.approx(100.0)


def test_calculate_ticker_indicators_returns_none_for_missing_or_short_data():
    calculator = TechnicalIndicatorCalculator()
    close_prices = pd.DataFrame({"AAPL": [1.0, 2.0]})

    assert calculator.calculate_ticker_indicators(
        "MSFT", close_prices, pd.DataFrame()) is None
    assert calculator.calculate_ticker_indicators(
        "AAPL", close_prices, pd.DataFrame()) is None


def test_latest_volume_returns_none_for_missing_or_empty_volume():
    calculator = TechnicalIndicatorCalculator()

    assert calculator.latest_volume("AAPL", pd.DataFrame()) is None
    assert calculator.latest_volume(
        "AAPL", pd.DataFrame({"AAPL": [None, pd.NA]})) is None
    assert calculator.latest_volume(
        "AAPL", pd.DataFrame({"AAPL": [None, 10]})) == 10
