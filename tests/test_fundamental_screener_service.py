import pandas as pd

from stock_screener.services.fundamental.fundamental_screener_service import (
    FundamentalScreenerService,
)
from stock_screener.utils.screener_rules import FUNDAMENTAL_SCORE_COLUMN


class FakeFundamentalClient:
    def __init__(self):
        self.calls = []

    def fetch(self, limit: int):
        self.calls.append(limit)
        return pd.DataFrame(
            [
                {"Ticker": "AAPL", "PEG": "2"},
                {"Ticker": "", "PEG": "3"},
            ]
        )


class FakeFundamentalNormalizer:
    def __init__(self):
        self.removed_input = None
        self.normalized_input = None

    def remove_invalid_rows(self, data: pd.DataFrame) -> pd.DataFrame:
        self.removed_input = data.copy()
        return data[data["Ticker"] != ""].reset_index(drop=True)

    def normalize(self, data: pd.DataFrame) -> pd.DataFrame:
        self.normalized_input = data.copy()
        normalized = data.copy()
        normalized["PEG"] = normalized["PEG"].astype(float)
        return normalized


class FakeFundamentalScoreCalculator:
    def __init__(self):
        self.calls = []

    def add_score(self, data: pd.DataFrame, columns: list[str]) -> pd.DataFrame:
        self.calls.append({"data": data.copy(), "columns": columns})
        scored = data.copy()
        scored[FUNDAMENTAL_SCORE_COLUMN] = 100.0
        return scored


def test_fundamental_screener_service_runs_fetch_normalize_and_score_pipeline():
    client = FakeFundamentalClient()
    normalizer = FakeFundamentalNormalizer()
    score_calculator = FakeFundamentalScoreCalculator()
    service = FundamentalScreenerService(
        client=client,
        normalizer=normalizer,
        score_calculator=score_calculator,
    )

    result = service.run(limit=25)

    assert client.calls == [25]
    assert normalizer.removed_input["Ticker"].tolist() == ["AAPL", ""]
    assert normalizer.normalized_input["Ticker"].tolist() == ["AAPL"]
    assert score_calculator.calls[0]["data"].loc[0, "PEG"] == 2.0
    assert result.to_dict("records") == [
        {"Ticker": "AAPL", "PEG": 2.0, FUNDAMENTAL_SCORE_COLUMN: 100.0}
    ]
    assert result is not score_calculator.calls[0]["data"]
