import pandas as pd

from stock_screener.services.integrated.potential_stock_filter import PotentialStockFilter


def _all_pass_stock(ticker, **overrides):
    """一隻基本面 + 技術全部過關嘅 baseline stock。"""
    data = {
        "Ticker": ticker,
        "Market Cap": 20_000_000_000,  # >= 15B
        "ROE": 0.20,  # >= 18%
        "EPS Past 5Y": 0.20,  # >= 15%
        "Sales Past 5Y": 0.20,  # >= 18%
        "Debt/Equity": 1.0,  # < 1.5
        "Gross Margin": 0.50,  # >= 40%
        "ROC125": 0.20,
        "EMA200Distance": 0.05,  # > 0 (Price > EMA200)
        "RSI14": 55.0,  # ∈ [40, 78]
    }
    data.update(overrides)
    return data


def _apply_one(**overrides):
    """喺獨立嘅單行 DataFrame 度行 filter，ROC125 percentile（單個有效值 = 100）
    唔會干擾測緊嗰個欄位。"""
    result = PotentialStockFilter().apply(
        pd.DataFrame([_all_pass_stock("T", **overrides)])
    )
    return bool(result.iloc[0])


def test_potential_stock_filter_baseline_passes():
    assert _apply_one() is True


def test_potential_stock_filter_fundamental_boundaries():
    # Market Cap >= 15B
    assert _apply_one(**{"Market Cap": 15_000_000_000}) is True
    assert _apply_one(**{"Market Cap": 14_999_999_999}) is False

    # ROE >= 18%
    assert _apply_one(**{"ROE": 0.18}) is True
    assert _apply_one(**{"ROE": 0.1799}) is False

    # EPS Past 5Y >= 15% OR Sales Past 5Y >= 18%
    assert _apply_one(**{"EPS Past 5Y": 0.15, "Sales Past 5Y": 0.10}) is True
    assert _apply_one(**{"EPS Past 5Y": 0.10, "Sales Past 5Y": 0.18}) is True
    assert _apply_one(**{"EPS Past 5Y": 0.149, "Sales Past 5Y": 0.179}) is False
    assert _apply_one(**{"EPS Past 5Y": pd.NA, "Sales Past 5Y": pd.NA}) is False
    # OR 語義：EPS 缺但 Sales 夠 -> 仍過增長關
    assert _apply_one(**{"EPS Past 5Y": pd.NA, "Sales Past 5Y": 0.20}) is True

    # Debt/Equity < 1.5
    assert _apply_one(**{"Debt/Equity": 1.49}) is True
    assert _apply_one(**{"Debt/Equity": 1.5}) is False

    # Gross Margin >= 40%
    assert _apply_one(**{"Gross Margin": 0.40}) is True
    assert _apply_one(**{"Gross Margin": 0.3999}) is False


def test_potential_stock_filter_technical_boundaries():
    # Price > EMA200 (EMA200Distance > 0)
    assert _apply_one(**{"EMA200Distance": 0.0001}) is True
    assert _apply_one(**{"EMA200Distance": 0.0}) is False
    assert _apply_one(**{"EMA200Distance": -0.0001}) is False

    # RSI14 ∈ [40, 78]（兩端 inclusive）
    assert _apply_one(**{"RSI14": 40.0}) is True
    assert _apply_one(**{"RSI14": 78.0}) is True
    assert _apply_one(**{"RSI14": 39.99}) is False
    assert _apply_one(**{"RSI14": 78.01}) is False

    # 缺任何一個輸入 -> False
    assert _apply_one(**{"ROE": pd.NA}) is False
    assert _apply_one(**{"Debt/Equity": pd.NA}) is False
    assert _apply_one(**{"Gross Margin": pd.NA}) is False
    assert _apply_one(**{"EMA200Distance": pd.NA}) is False
    assert _apply_one(**{"RSI14": pd.NA}) is False
    assert _apply_one(**{"ROC125": pd.NA}) is False


def test_potential_stock_filter_requires_both_fundamental_and_technical():
    # 基本面過但技術唔過 -> False
    assert _apply_one(**{"RSI14": 90.0}) is False
    # 技術過但基本面唔過 -> False
    assert _apply_one(**{"Market Cap": 1_000_000_000}) is False


def test_potential_stock_filter_roc125_uses_whole_market_percentile():
    # 6 隻全部基本面 + EMA + RSI 過關，淨係 ROC125 唔同。
    # percentile_score 對 6 個升序值：0, 20, 40, 60, 80, 100。
    # >= 60th percentile -> 末尾三隻（60/80/100）過。
    roc_values = [0.01, 0.03, 0.06, 0.10, 0.20, 0.50]
    rows = [
        _all_pass_stock(f"T{i}", **{"ROC125": roc})
        for i, roc in enumerate(roc_values)
    ]
    result = PotentialStockFilter().apply(pd.DataFrame(rows))

    assert result.tolist() == [False, False, False, True, True, True]


def test_potential_stock_filter_missing_all_inputs_false():
    result = PotentialStockFilter().apply(
        pd.DataFrame([{"Ticker": "EMPTY", "P/S": 9.0}])
    )

    assert result.tolist() == [False]
    assert result.dtype == bool
