import pandas as pd

from stock_screener.services.integrated.potential_stock_filter import PotentialStockFilter


def _all_pass_stock(ticker, **overrides):
    """一隻 AI 半導體 quality momentum 條件全部過關嘅 baseline stock。"""
    data = {
        "Ticker": ticker,
        "Market Cap": 20_000_000_000,  # >= 10B
        "Volume": 2_000_000,  # >= 1M
        "Forward P/E": 25.0,  # 1-60
        "PEG": 1.0,  # 0.01-1.5
        "P/FCF": 50.0,  # 1-180
        "ROE": 0.20,  # >= 15%
        "ROIC": 0.15,  # >= 10%
        "Profit Margin": 0.20,  # >= 10%
        "EPS Past 5Y": 0.20,  # >= 10%
        "Sales Past 5Y": 0.20,  # >= 10%
        "Debt/Equity": 1.0,  # <= 1.5
        "ROC125": 0.20,
        "ROC20": 0.05,  # > -15%
        "EMA200Distance": 0.05,  # > 0 (Price > EMA200)
        "RSI14": 55.0,  # ∈ [35, 75]
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
    # Market Cap >= 10B
    assert _apply_one(**{"Market Cap": 10_000_000_000}) is True
    assert _apply_one(**{"Market Cap": 9_999_999_999}) is False

    # Volume >= 1M
    assert _apply_one(**{"Volume": 1_000_000}) is True
    assert _apply_one(**{"Volume": 999_999}) is False

    # Forward P/E between 1 and 60
    assert _apply_one(**{"Forward P/E": 1.0}) is True
    assert _apply_one(**{"Forward P/E": 60.0}) is True
    assert _apply_one(**{"Forward P/E": 0.99}) is False
    assert _apply_one(**{"Forward P/E": 60.01}) is False

    # PEG between 0.01 and 1.5
    assert _apply_one(**{"PEG": 0.01}) is True
    assert _apply_one(**{"PEG": 1.5}) is True
    assert _apply_one(**{"PEG": 0.009}) is False
    assert _apply_one(**{"PEG": 1.501}) is False

    # P/FCF between 1 and 180
    assert _apply_one(**{"P/FCF": 1.0}) is True
    assert _apply_one(**{"P/FCF": 180.0}) is True
    assert _apply_one(**{"P/FCF": 0.99}) is False
    assert _apply_one(**{"P/FCF": 180.01}) is False

    # ROE >= 15%
    assert _apply_one(**{"ROE": 0.15}) is True
    assert _apply_one(**{"ROE": 0.1499}) is False

    # ROIC >= 10%
    assert _apply_one(**{"ROIC": 0.10}) is True
    assert _apply_one(**{"ROIC": 0.0999}) is False

    # Profit Margin >= 10%
    assert _apply_one(**{"Profit Margin": 0.10}) is True
    assert _apply_one(**{"Profit Margin": 0.0999}) is False

    # EPS Past 5Y >= 10% AND Sales Past 5Y >= 10%
    assert _apply_one(**{"EPS Past 5Y": 0.10, "Sales Past 5Y": 0.10}) is True
    assert _apply_one(**{"EPS Past 5Y": 0.099, "Sales Past 5Y": 0.20}) is False
    assert _apply_one(**{"EPS Past 5Y": 0.20, "Sales Past 5Y": 0.099}) is False
    assert _apply_one(**{"EPS Past 5Y": pd.NA, "Sales Past 5Y": pd.NA}) is False
    assert _apply_one(**{"EPS Past 5Y": pd.NA, "Sales Past 5Y": 0.20}) is False

    # Debt/Equity <= 1.5
    assert _apply_one(**{"Debt/Equity": 1.5}) is True
    assert _apply_one(**{"Debt/Equity": 1.5001}) is False


def test_potential_stock_filter_technical_boundaries():
    # ROC125 between 10% and 400%
    assert _apply_one(**{"ROC125": 0.10}) is True
    assert _apply_one(**{"ROC125": 4.0}) is True
    assert _apply_one(**{"ROC125": 0.099}) is False
    assert _apply_one(**{"ROC125": 4.001}) is False

    # ROC20 > -15%
    assert _apply_one(**{"ROC20": -0.149}) is True
    assert _apply_one(**{"ROC20": -0.15}) is False

    # Price > EMA200 (EMA200Distance > 0)
    assert _apply_one(**{"EMA200Distance": 0.0001}) is True
    assert _apply_one(**{"EMA200Distance": 0.0}) is False
    assert _apply_one(**{"EMA200Distance": -0.0001}) is False

    # RSI14 ∈ [35, 75]（兩端 inclusive）
    assert _apply_one(**{"RSI14": 35.0}) is True
    assert _apply_one(**{"RSI14": 75.0}) is True
    assert _apply_one(**{"RSI14": 34.99}) is False
    assert _apply_one(**{"RSI14": 75.01}) is False

    # 缺任何一個輸入 -> False
    assert _apply_one(**{"ROE": pd.NA}) is False
    assert _apply_one(**{"Debt/Equity": pd.NA}) is False
    assert _apply_one(**{"Forward P/E": pd.NA}) is False
    assert _apply_one(**{"PEG": pd.NA}) is False
    assert _apply_one(**{"P/FCF": pd.NA}) is False
    assert _apply_one(**{"ROIC": pd.NA}) is False
    assert _apply_one(**{"Profit Margin": pd.NA}) is False
    assert _apply_one(**{"EMA200Distance": pd.NA}) is False
    assert _apply_one(**{"ROC20": pd.NA}) is False
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
