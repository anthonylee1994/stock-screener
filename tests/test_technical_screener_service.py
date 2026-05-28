from stock_screener.services.technical.technical_score_calculator import (
    DEFAULT_COLUMNS as TECHNICAL_DEFAULT_COLUMNS,
)
from stock_screener.services.technical.technical_screener_service import (
    TechnicalScreenerService,
)


class FakeTechnicalPriceClient:
    def __init__(self):
        self.calls = []

    def download_price_data(self, tickers: list[str]):
        self.calls.append(tickers)
        return {"price": "data"}


class FakeEmptyTechnicalIndicatorCalculator:
    def __init__(self):
        self.calls = []

    def calculate_indicators(self, price_data, tickers: list[str]):
        self.calls.append({"price_data": price_data, "tickers": tickers})
        import pandas as pd

        return pd.DataFrame()


class FakeTechnicalScoreCalculator:
    def add_scores(self, data):
        scored = data.copy()
        scored["Technical Score"] = 100.0
        return scored


def test_technical_screener_service_returns_empty_default_columns_for_no_tickers():
    service = TechnicalScreenerService()

    result = service.run(["", " "])

    assert result.empty
    assert result.columns.tolist() == TECHNICAL_DEFAULT_COLUMNS


class FailingTechnicalPriceClient:
    def download_price_data(self, tickers: list[str]):
        raise RuntimeError("download failed")


def test_technical_screener_service_returns_empty_default_columns_on_price_error():
    service = TechnicalScreenerService(
        price_client=FailingTechnicalPriceClient())

    result = service.run(["aapl"])

    assert result.empty
    assert result.columns.tolist() == TECHNICAL_DEFAULT_COLUMNS


def test_technical_screener_service_returns_empty_default_columns_for_empty_indicators():
    price_client = FakeTechnicalPriceClient()
    indicator_calculator = FakeEmptyTechnicalIndicatorCalculator()
    service = TechnicalScreenerService(
        price_client=price_client,
        indicator_calculator=indicator_calculator,
    )

    result = service.run([" aapl ", "AAPL", "msft"])

    assert price_client.calls == [["AAPL", "MSFT"]]
    assert indicator_calculator.calls == [
        {"price_data": {"price": "data"}, "tickers": ["AAPL", "MSFT"]}
    ]
    assert result.empty
    assert result.columns.tolist() == TECHNICAL_DEFAULT_COLUMNS


def test_technical_screener_service_scores_indicator_data_and_returns_copy():
    import pandas as pd

    class IndicatorCalculator:
        def calculate_indicators(self, price_data, tickers: list[str]):
            return pd.DataFrame([{"Ticker": "AAPL"}])

    service = TechnicalScreenerService(
        price_client=FakeTechnicalPriceClient(),
        indicator_calculator=IndicatorCalculator(),
        score_calculator=FakeTechnicalScoreCalculator(),
    )

    result = service.run(["AAPL"])

    assert result.to_dict("records") == [
        {"Ticker": "AAPL", "Technical Score": 100.0}]
