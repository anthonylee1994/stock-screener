import pandas as pd

from stock_screener.services.common.percentile_scorer import percentile_score
from stock_screener.services.common.score_curver import curve_score
from stock_screener.services.fundamental.fundamental_data_normalizer import (
    FundamentalDataNormalizer,
)
from stock_screener.services.fundamental.fundamental_score_calculator import (
    FundamentalScoreCalculator,
    SCORE_WEIGHTS,
)
from stock_screener.services.technical.technical_score_calculator import (
    TechnicalScoreCalculator,
)


def test_fundamental_data_normalizer_rounds_non_score_metrics():
    normalizer = FundamentalDataNormalizer()
    data = pd.DataFrame(
        [
            {
                "Market Cap": "1234.567",
                "Forward P/E": "25.555",
                "PEG": "1.234",
                "P/S": "5.678",
                "P/FCF": "44.444",
                "EPS Past 5Y": "12.345%",
                "EPS Q/Q": "40%",
                "Sales Past 5Y": "6.789%",
                "Sales Q/Q": "14%",
                "ROE": "10.456%",
                "ROIC": "9.876%",
                "Profit Margin": "20.129%",
                "Gross M": "60.5%",
                "Oper M": "12.5%",
                "Debt/Equity": "0.876",
                "Short Float": "3.5%",
                "SMA200": "4.5%",
                "52W High": "-8%",
                "Target Price": "250.50",
                "Price": "200.25",
            }
        ]
    )

    normalized = normalizer.normalize(data)

    assert normalized.loc[0, "Market Cap"] == 1234.567
    assert normalized.loc[0, "Forward P/E"] == 25.555
    assert normalized.loc[0, "PEG"] == 1.234
    assert normalized.loc[0, "P/S"] == 5.678
    assert normalized.loc[0, "P/FCF"] == 44.444
    assert normalized.loc[0, "EPS Past 5Y"] == 0.1234
    assert normalized.loc[0, "EPS Quarter Over Quarter"] == 0.4
    assert normalized.loc[0, "Sales Past 5Y"] == 0.0679
    assert normalized.loc[0, "Sales Quarter Over Quarter"] == 0.14
    assert normalized.loc[0, "ROE"] == 0.1046
    assert normalized.loc[0, "ROIC"] == 0.0988
    assert normalized.loc[0, "Profit Margin"] == 0.2013
    assert normalized.loc[0, "Gross Margin"] == 0.605
    assert normalized.loc[0, "Operating Margin"] == 0.125
    assert normalized.loc[0, "Debt/Equity"] == 0.876
    assert normalized.loc[0, "Short Interest"] == 0.035
    assert normalized.loc[0, "200-Day Simple Moving Average"] == 0.045
    assert normalized.loc[0, "52W High"] == -0.08
    assert normalized.loc[0, "Target Price"] == 250.5
    assert normalized.loc[0, "Target Price Upside"] == 0.2509


def test_fundamental_data_normalizer_returns_empty_data_unchanged():
    normalizer = FundamentalDataNormalizer()
    data = pd.DataFrame()

    result = normalizer.normalize(data)

    assert result is data


def test_fundamental_data_normalizer_remove_invalid_rows_handles_empty_and_missing_ticker():
    normalizer = FundamentalDataNormalizer()
    empty_data = pd.DataFrame()
    data_without_ticker = pd.DataFrame([{"Company": "Apple"}])

    assert normalizer.remove_invalid_rows(empty_data) is empty_data
    assert normalizer.remove_invalid_rows(data_without_ticker).to_dict("records") == [
        {"Company": "Apple"}
    ]


def test_fundamental_data_normalizer_remove_invalid_rows_strips_blank_tickers():
    normalizer = FundamentalDataNormalizer()
    data = pd.DataFrame(
        [
            {"Ticker": "AAPL"},
            {"Ticker": " "},
            {"Ticker": None},
        ]
    )

    result = normalizer.remove_invalid_rows(data)

    assert result.to_dict("records") == [{"Ticker": "AAPL"}]


def test_fundamental_score_calculator_scores_and_sorts_weighted_metrics():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "WEAK",
                "Market Cap": "1,000",
                "ROE": "10%",
                "Gross Margin": "20%",
                "EPS Past 5Y": "5%",
                "Sales Past 5Y": "2%",
                "Debt/Equity": "3",
                "PEG": "3",
            },
            {
                "Ticker": "STRONG",
                "Market Cap": "3,000",
                "ROE": "30%",
                "Gross Margin": "60%",
                "EPS Past 5Y": "25%",
                "Sales Past 5Y": "20%",
                "Debt/Equity": "0.2",
                "PEG": "1",
            },
        ]
    )
    columns = [
        "Ticker",
        "Market Cap Score",
        "ROE Score",
        "Gross Margin Score",
        "EPS Past 5Y Score",
        "PEG Score",
        "Fundamental Score",
    ]

    scored = calculator.add_score(data, columns)

    assert scored["Ticker"].tolist() == ["STRONG", "WEAK"]
    assert scored.loc[0, "Fundamental Score"] == 100.0
    assert scored.loc[1, "Fundamental Score"] == 0.0
    assert scored.loc[0, "ROE Score"] == 100.0
    assert scored.loc[0, "PEG Score"] == 100.0


def test_curve_score_scales_valid_scores_to_zero_and_one_hundred():
    score = pd.Series([10.0, 20.0, 30.0, pd.NA])

    curved = curve_score(score)

    assert curved.tolist()[:3] == [0.0, 50.0, 100.0]
    assert pd.isna(curved.iloc[3])


def test_curve_score_returns_one_hundred_for_single_or_flat_valid_score():
    assert curve_score(pd.Series([pd.NA, 42.0])).tolist()[1] == 100.0
    assert curve_score(pd.Series([7.0, 7.0])).tolist() == [100.0, 100.0]


def test_fundamental_score_weights_are_balanced_and_sum_to_one():
    assert sum(SCORE_WEIGHTS.values()) == 1.0
    assert SCORE_WEIGHTS == {
        "Market Cap": 0,
        "ROE": 0.22,
        "Gross Margin": 0.18,
        "EPS Past 5Y": 0.22,
        "Sales Past 5Y": 0.13,
        "Debt/Equity": 0.15,
        "PEG": 0.10,
    }


def test_fundamental_score_calculator_scores_metrics_relative_to_sector():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {"Ticker": "TECH_LOW", "Sector": "Technology", "ROE": 10},
            {"Ticker": "TECH_HIGH", "Sector": "Technology", "ROE": 20},
            {"Ticker": "TECH_MID", "Sector": "Technology", "ROE": 15},
            {"Ticker": "TECH_TOP", "Sector": "Technology", "ROE": 25},
            {"Ticker": "TECH_BOTTOM", "Sector": "Technology", "ROE": 5},
            {"Ticker": "ENERGY_LOW", "Sector": "Energy", "ROE": 100},
            {"Ticker": "ENERGY_HIGH", "Sector": "Energy", "ROE": 500},
            {"Ticker": "ENERGY_MID", "Sector": "Energy", "ROE": 300},
            {"Ticker": "ENERGY_TOP", "Sector": "Energy", "ROE": 700},
            {"Ticker": "ENERGY_BOTTOM", "Sector": "Energy", "ROE": 50},
        ]
    )

    scored = calculator.add_score(
        data, ["Ticker", "ROE Score", "Fundamental Score"])
    score_by_ticker = scored.set_index("Ticker")["ROE Score"].to_dict()

    assert score_by_ticker["TECH_TOP"] == 100.0
    assert score_by_ticker["TECH_BOTTOM"] == 0.0
    assert score_by_ticker["ENERGY_TOP"] == 100.0
    assert score_by_ticker["ENERGY_BOTTOM"] == 0.0
    assert score_by_ticker["TECH_TOP"] == score_by_ticker["ENERGY_TOP"]


def test_fundamental_score_calculator_falls_back_to_global_score_for_small_sector():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {"Ticker": "TECH_LOW", "Sector": "Technology", "ROE": 10},
            {"Ticker": "TECH_HIGH", "Sector": "Technology", "ROE": 20},
            {"Ticker": "TECH_MID", "Sector": "Technology", "ROE": 15},
            {"Ticker": "TECH_TOP", "Sector": "Technology", "ROE": 25},
            {"Ticker": "TECH_BOTTOM", "Sector": "Technology", "ROE": 5},
            {"Ticker": "HEALTH_ONLY", "Sector": " Healthcare ", "ROE": 100},
            {"Ticker": "MISSING_SECTOR", "Sector": " ", "ROE": 50},
        ]
    )

    scored = calculator.add_score(data, ["Ticker", "ROE Score"])
    score_by_ticker = scored.set_index("Ticker")["ROE Score"].to_dict()

    assert score_by_ticker["HEALTH_ONLY"] == 100.0
    assert score_by_ticker["MISSING_SECTOR"] == 83.33


def test_fundamental_score_calculator_caps_score_when_core_metrics_are_missing():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "INSUFFICIENT_CORE",
                "Sector": "Technology",
                "ROE": 100,
                "Gross Margin": pd.NA,
                "EPS Past 5Y": pd.NA,
                "Sales Past 5Y": 100,
                "Debt/Equity": 0,
            },
            {
                "Ticker": "HAS_CORE",
                "Sector": "Technology",
                "ROE": 90,
                "Gross Margin": 90,
                "EPS Past 5Y": 90,
                "Sales Past 5Y": 90,
                "Debt/Equity": 0.1,
            },
            {
                "Ticker": "PEER_LOW",
                "Sector": "Technology",
                "ROE": 1,
                "Gross Margin": 1,
                "EPS Past 5Y": 1,
                "Sales Past 5Y": 1,
                "Debt/Equity": 5,
            },
            {
                "Ticker": "PEER_MID",
                "Sector": "Technology",
                "ROE": 2,
                "Gross Margin": 2,
                "EPS Past 5Y": 2,
                "Sales Past 5Y": 2,
                "Debt/Equity": 4,
            },
            {
                "Ticker": "PEER_HIGH",
                "Sector": "Technology",
                "ROE": 3,
                "Gross Margin": 3,
                "EPS Past 5Y": 3,
                "Sales Past 5Y": 3,
                "Debt/Equity": 3,
            },
        ]
    )

    scored = calculator.add_score(data, ["Ticker", "Fundamental Score"])
    score_by_ticker = scored.set_index("Ticker")["Fundamental Score"].to_dict()

    assert score_by_ticker["INSUFFICIENT_CORE"] == 60.0
    assert score_by_ticker["HAS_CORE"] > 60.0


def test_fundamental_score_calculator_caps_score_when_core_scores_are_weak():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "WEAK_CORE",
                "Sector": "Technology",
                "ROE": 10,
                "Gross Margin": 10,
                "EPS Past 5Y": 10,
                "Sales Past 5Y": 100,
                "Debt/Equity": 0,
            },
            {
                "Ticker": "STRONG_CORE",
                "Sector": "Technology",
                "ROE": 100,
                "Gross Margin": 100,
                "EPS Past 5Y": 100,
                "Sales Past 5Y": 90,
                "Debt/Equity": 0.1,
            },
            {
                "Ticker": "PEER_LOW",
                "Sector": "Technology",
                "ROE": 5,
                "Gross Margin": 5,
                "EPS Past 5Y": 5,
                "Sales Past 5Y": 5,
                "Debt/Equity": 5,
            },
            {
                "Ticker": "PEER_MID",
                "Sector": "Technology",
                "ROE": 30,
                "Gross Margin": 30,
                "EPS Past 5Y": 30,
                "Sales Past 5Y": 30,
                "Debt/Equity": 3,
            },
            {
                "Ticker": "PEER_HIGH",
                "Sector": "Technology",
                "ROE": 50,
                "Gross Margin": 50,
                "EPS Past 5Y": 50,
                "Sales Past 5Y": 50,
                "Debt/Equity": 1,
            },
        ]
    )

    scored = calculator.add_score(data, ["Ticker", "Fundamental Score"])
    score_by_ticker = scored.set_index("Ticker")["Fundamental Score"].to_dict()

    assert score_by_ticker["WEAK_CORE"] <= 75.0
    assert score_by_ticker["STRONG_CORE"] > 75.0


def test_fundamental_score_calculator_caps_score_when_quality_is_weak():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "WEAK_QUALITY",
                "Sector": "Technology",
                "ROE": 2,
                "Gross Margin": 2,
                "EPS Past 5Y": 100,
                "Sales Past 5Y": 100,
                "Debt/Equity": 0,
            },
            {
                "Ticker": "STRONG_QUALITY",
                "Sector": "Technology",
                "ROE": 100,
                "Gross Margin": 100,
                "EPS Past 5Y": 90,
                "Sales Past 5Y": 90,
                "Debt/Equity": 0.1,
            },
            {
                "Ticker": "PEER_LOW",
                "Sector": "Technology",
                "ROE": 1,
                "Gross Margin": 1,
                "EPS Past 5Y": 1,
                "Sales Past 5Y": 1,
                "Debt/Equity": 5,
            },
            {
                "Ticker": "PEER_MID",
                "Sector": "Technology",
                "ROE": 5,
                "Gross Margin": 5,
                "EPS Past 5Y": 5,
                "Sales Past 5Y": 5,
                "Debt/Equity": 3,
            },
            {
                "Ticker": "PEER_HIGH",
                "Sector": "Technology",
                "ROE": 50,
                "Gross Margin": 50,
                "EPS Past 5Y": 50,
                "Sales Past 5Y": 50,
                "Debt/Equity": 1,
            },
        ]
    )

    scored = calculator.add_score(data, ["Ticker", "Fundamental Score"])
    score_by_ticker = scored.set_index("Ticker")["Fundamental Score"].to_dict()

    assert score_by_ticker["WEAK_QUALITY"] == 70.0
    assert score_by_ticker["STRONG_QUALITY"] > 70.0


def test_fundamental_score_calculator_caps_score_when_peg_is_stretched():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "EXPENSIVE_GROWTH",
                "Sector": "Technology",
                "ROE": 100,
                "Gross Margin": 100,
                "EPS Past 5Y": 100,
                "Sales Past 5Y": 100,
                "Debt/Equity": 0,
                "PEG": 10,
            },
            {
                "Ticker": "REASONABLE_GROWTH",
                "Sector": "Technology",
                "ROE": 90,
                "Gross Margin": 90,
                "EPS Past 5Y": 90,
                "Sales Past 5Y": 90,
                "Debt/Equity": 0.1,
                "PEG": 0.5,
            },
            {
                "Ticker": "PEER_LOW",
                "Sector": "Technology",
                "ROE": 1,
                "Gross Margin": 1,
                "EPS Past 5Y": 1,
                "Sales Past 5Y": 1,
                "Debt/Equity": 5,
                "PEG": 5,
            },
            {
                "Ticker": "PEER_MID",
                "Sector": "Technology",
                "ROE": 2,
                "Gross Margin": 2,
                "EPS Past 5Y": 2,
                "Sales Past 5Y": 2,
                "Debt/Equity": 4,
                "PEG": 4,
            },
            {
                "Ticker": "PEER_HIGH",
                "Sector": "Technology",
                "ROE": 3,
                "Gross Margin": 3,
                "EPS Past 5Y": 3,
                "Sales Past 5Y": 3,
                "Debt/Equity": 3,
                "PEG": 3,
            },
        ]
    )

    scored = calculator.add_score(data, ["Ticker", "Fundamental Score"])
    score_by_ticker = scored.set_index("Ticker")["Fundamental Score"].to_dict()

    assert score_by_ticker["EXPENSIVE_GROWTH"] == 85.0
    assert score_by_ticker["REASONABLE_GROWTH"] > 85.0


def test_fundamental_score_calculator_curves_final_score():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "LOW",
                "ROE": 10,
                "Gross Margin": 10,
                "EPS Past 5Y": 10,
                "Sales Past 5Y": 10,
                "Debt/Equity": 50,
                "PEG": 3,
            },
            {
                "Ticker": "MID",
                "ROE": 20,
                "Gross Margin": 20,
                "EPS Past 5Y": 20,
                "Sales Past 5Y": 20,
                "Debt/Equity": 30,
                "PEG": 2,
            },
            {
                "Ticker": "HIGH",
                "ROE": 30,
                "Gross Margin": 30,
                "EPS Past 5Y": 30,
                "Sales Past 5Y": 30,
                "Debt/Equity": 10,
                "PEG": 1,
            },
        ]
    )

    scored = calculator.add_score(data, ["Ticker", "Fundamental Score"])

    assert scored["Fundamental Score"].tolist() == [100.0, 50.0, 0.0]


def test_fundamental_score_calculator_handles_no_scoreable_columns():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame([{"Ticker": "AAPL"}])

    scored = calculator.add_score(data, ["Ticker", "Fundamental Score"])

    assert scored.to_dict("records") == [
        {"Ticker": "AAPL", "Fundamental Score": 0.0}
    ]


def test_fundamental_score_calculator_returns_empty_data_unchanged():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame()

    result = calculator.add_score(data, ["Ticker"])

    assert result is data


def test_technical_score_calculator_scores_percentiles_and_sorts():
    calculator = TechnicalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "LOW",
                "Quote Price": 10.0,
                "Quote Change": 0.1,
                "Quote Change Percent": 1.0,
                "Quote Volume": 1_000_000,
                "EMA200Distance": 1,
                "ROC125": 1,
                "EMA50Distance": 1,
                "ROC20": 1,
                "PPO Slope3": 1,
                "RSI14": 1,
            },
            {
                "Ticker": "HIGH",
                "Quote Price": 20.0,
                "Quote Change": 0.2,
                "Quote Change Percent": 2.0,
                "Quote Volume": 2_000_000,
                "EMA200Distance": 2,
                "ROC125": 2,
                "EMA50Distance": 2,
                "ROC20": 2,
                "PPO Slope3": 2,
                "RSI14": 2,
            },
        ]
    )

    scored = calculator.add_scores(data)

    assert scored["Ticker"].tolist() == ["HIGH", "LOW"]
    assert scored.loc[0, "Long Term Score"] == 100.0
    assert scored.loc[0, "Mid Term Score"] == 100.0
    assert scored.loc[0, "Short Term Score"] == 100.0
    assert scored.loc[0, "Technical Score"] == 100.0


def test_technical_score_calculator_curves_final_score():
    calculator = TechnicalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "LOW",
                "Quote Price": 10.0,
                "Quote Change": 0.1,
                "Quote Change Percent": 1.0,
                "Quote Volume": 1_000_000,
                "EMA200Distance": 1,
                "ROC125": 1,
                "EMA50Distance": 1,
                "ROC20": 1,
                "PPO Slope3": 1,
                "RSI14": 1,
            },
            {
                "Ticker": "MID",
                "Quote Price": 15.0,
                "Quote Change": 0.15,
                "Quote Change Percent": 1.5,
                "Quote Volume": 1_500_000,
                "EMA200Distance": 2,
                "ROC125": 2,
                "EMA50Distance": 2,
                "ROC20": 2,
                "PPO Slope3": 2,
                "RSI14": 2,
            },
            {
                "Ticker": "HIGH",
                "Quote Price": 20.0,
                "Quote Change": 0.2,
                "Quote Change Percent": 2.0,
                "Quote Volume": 2_000_000,
                "EMA200Distance": 3,
                "ROC125": 3,
                "EMA50Distance": 3,
                "ROC20": 3,
                "PPO Slope3": 3,
                "RSI14": 3,
            },
        ]
    )

    scored = calculator.add_scores(data)

    assert scored["Ticker"].tolist() == ["HIGH", "MID", "LOW"]
    assert scored["Technical Score"].tolist() == [100.0, 50.0, 0.0]


def test_technical_percentile_score_handles_all_missing_and_single_valid_value():
    all_missing = percentile_score(pd.Series([pd.NA, None]), ascending=True)
    single_valid = percentile_score(pd.Series([pd.NA, 42.0]), ascending=True)

    assert all_missing.tolist() == [0.0, 0.0]
    assert single_valid.tolist() == [0.0, 100.0]
