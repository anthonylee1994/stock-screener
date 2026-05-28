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
                "Sales Past 5Y": "6.789%",
                "ROE": "10.456%",
                "ROIC": "9.876%",
                "Profit Margin": "20.129%",
                "Debt/Equity": "0.876",
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
    assert normalized.loc[0, "Sales Past 5Y"] == 0.0679
    assert normalized.loc[0, "ROE"] == 10.456
    assert normalized.loc[0, "ROIC"] == 0.0988
    assert normalized.loc[0, "Profit Margin"] == 20.129
    assert normalized.loc[0, "Debt/Equity"] == 0.876


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
                "PEG": "3",
                "P/S": "10",
                "P/FCF": "50",
                "Forward P/E": "40",
                "EPS Past 5Y": "5%",
                "Sales Past 5Y": "2%",
                "ROE": "10%",
                "ROIC": "10%",
                "Profit Margin": "5%",
                "Debt/Equity": "3",
            },
            {
                "Ticker": "STRONG",
                "Market Cap": "3,000",
                "PEG": "1",
                "P/S": "2",
                "P/FCF": "10",
                "Forward P/E": "15",
                "EPS Past 5Y": "25%",
                "Sales Past 5Y": "20%",
                "ROE": "30%",
                "ROIC": "35%",
                "Profit Margin": "25%",
                "Debt/Equity": "0.2",
            },
        ]
    )
    columns = [
        "Ticker",
        "Market Cap Score",
        "PEG Score",
        "EPS Past 5Y Score",
        "ROE Score",
        "ROIC Score",
        "Fundamental Score",
    ]

    scored = calculator.add_score(data, columns)

    assert scored["Ticker"].tolist() == ["STRONG", "WEAK"]
    assert scored.loc[0, "Fundamental Score"] == 100.0
    assert scored.loc[1, "Fundamental Score"] == 0.0
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
        "EPS Past 5Y": 0.18,
        "Sales Past 5Y": 0.08,
        "ROE": 0.13,
        "ROIC": 0.2,
        "Profit Margin": 0.06,
        "Forward P/E": 0.05,
        "PEG": 0.17,
        "P/S": 0.03,
        "P/FCF": 0.07,
        "Debt/Equity": 0.03,
    }


def test_fundamental_score_calculator_scores_metrics_relative_to_sector():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {"Ticker": "TECH_LOW", "Sector": "Technology", "ROIC": 10},
            {"Ticker": "TECH_HIGH", "Sector": "Technology", "ROIC": 20},
            {"Ticker": "TECH_MID", "Sector": "Technology", "ROIC": 15},
            {"Ticker": "TECH_TOP", "Sector": "Technology", "ROIC": 25},
            {"Ticker": "TECH_BOTTOM", "Sector": "Technology", "ROIC": 5},
            {"Ticker": "ENERGY_LOW", "Sector": "Energy", "ROIC": 100},
            {"Ticker": "ENERGY_HIGH", "Sector": "Energy", "ROIC": 500},
            {"Ticker": "ENERGY_MID", "Sector": "Energy", "ROIC": 300},
            {"Ticker": "ENERGY_TOP", "Sector": "Energy", "ROIC": 700},
            {"Ticker": "ENERGY_BOTTOM", "Sector": "Energy", "ROIC": 50},
        ]
    )

    scored = calculator.add_score(data, ["Ticker", "ROIC Score", "Fundamental Score"])
    score_by_ticker = scored.set_index("Ticker")["ROIC Score"].to_dict()

    assert score_by_ticker["TECH_TOP"] == 100.0
    assert score_by_ticker["TECH_BOTTOM"] == 0.0
    assert score_by_ticker["ENERGY_TOP"] == 100.0
    assert score_by_ticker["ENERGY_BOTTOM"] == 0.0
    assert score_by_ticker["TECH_TOP"] == score_by_ticker["ENERGY_TOP"]


def test_fundamental_score_calculator_falls_back_to_global_score_for_small_sector():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {"Ticker": "TECH_LOW", "Sector": "Technology", "ROIC": 10},
            {"Ticker": "TECH_HIGH", "Sector": "Technology", "ROIC": 20},
            {"Ticker": "TECH_MID", "Sector": "Technology", "ROIC": 15},
            {"Ticker": "TECH_TOP", "Sector": "Technology", "ROIC": 25},
            {"Ticker": "TECH_BOTTOM", "Sector": "Technology", "ROIC": 5},
            {"Ticker": "HEALTH_ONLY", "Sector": " Healthcare ", "ROIC": 100},
            {"Ticker": "MISSING_SECTOR", "Sector": " ", "ROIC": 50},
        ]
    )

    scored = calculator.add_score(data, ["Ticker", "ROIC Score"])
    score_by_ticker = scored.set_index("Ticker")["ROIC Score"].to_dict()

    assert score_by_ticker["HEALTH_ONLY"] == 100.0
    assert score_by_ticker["MISSING_SECTOR"] == 83.33


def test_fundamental_score_calculator_caps_score_when_core_metrics_are_missing():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "INSUFFICIENT_CORE",
                "Sector": "Technology",
                "ROIC": pd.NA,
                "EPS Past 5Y": pd.NA,
                "PEG": 0.5,
                "ROE": 100,
                "Sales Past 5Y": 100,
                "Profit Margin": 100,
                "P/FCF": 1,
                "Forward P/E": 1,
                "P/S": 1,
                "Debt/Equity": 0,
            },
            {
                "Ticker": "HAS_CORE",
                "Sector": "Technology",
                "ROIC": 100,
                "EPS Past 5Y": 100,
                "PEG": pd.NA,
                "ROE": 90,
                "Sales Past 5Y": 90,
                "Profit Margin": 90,
                "P/FCF": 2,
                "Forward P/E": 2,
                "P/S": 2,
                "Debt/Equity": 0.1,
            },
            {
                "Ticker": "PEER_LOW",
                "Sector": "Technology",
                "ROIC": 1,
                "EPS Past 5Y": 1,
                "PEG": 5,
                "ROE": 1,
                "Sales Past 5Y": 1,
                "Profit Margin": 1,
                "P/FCF": 50,
                "Forward P/E": 50,
                "P/S": 50,
                "Debt/Equity": 5,
            },
            {
                "Ticker": "PEER_MID",
                "Sector": "Technology",
                "ROIC": 2,
                "EPS Past 5Y": 2,
                "PEG": 4,
                "ROE": 2,
                "Sales Past 5Y": 2,
                "Profit Margin": 2,
                "P/FCF": 40,
                "Forward P/E": 40,
                "P/S": 40,
                "Debt/Equity": 4,
            },
            {
                "Ticker": "PEER_HIGH",
                "Sector": "Technology",
                "ROIC": 3,
                "EPS Past 5Y": 3,
                "PEG": 3,
                "ROE": 3,
                "Sales Past 5Y": 3,
                "Profit Margin": 3,
                "P/FCF": 30,
                "Forward P/E": 30,
                "P/S": 30,
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
                "ROIC": 35,
                "EPS Past 5Y": 35,
                "PEG": 0.1,
                "ROE": 100,
                "Sales Past 5Y": 100,
                "Profit Margin": 100,
                "P/FCF": 1,
                "Forward P/E": 1,
                "P/S": 1,
                "Debt/Equity": 0,
            },
            {
                "Ticker": "STRONG_CORE",
                "Sector": "Technology",
                "ROIC": 100,
                "EPS Past 5Y": 100,
                "PEG": 0.2,
                "ROE": 90,
                "Sales Past 5Y": 90,
                "Profit Margin": 90,
                "P/FCF": 2,
                "Forward P/E": 2,
                "P/S": 2,
                "Debt/Equity": 0.1,
            },
            {
                "Ticker": "PEER_LOW",
                "Sector": "Technology",
                "ROIC": 10,
                "EPS Past 5Y": 10,
                "PEG": 5,
                "ROE": 1,
                "Sales Past 5Y": 1,
                "Profit Margin": 1,
                "P/FCF": 50,
                "Forward P/E": 50,
                "P/S": 50,
                "Debt/Equity": 5,
            },
            {
                "Ticker": "PEER_MID",
                "Sector": "Technology",
                "ROIC": 30,
                "EPS Past 5Y": 30,
                "PEG": 4,
                "ROE": 2,
                "Sales Past 5Y": 2,
                "Profit Margin": 2,
                "P/FCF": 40,
                "Forward P/E": 40,
                "P/S": 40,
                "Debt/Equity": 4,
            },
            {
                "Ticker": "PEER_HIGH",
                "Sector": "Technology",
                "ROIC": 40,
                "EPS Past 5Y": 40,
                "PEG": 3,
                "ROE": 3,
                "Sales Past 5Y": 3,
                "Profit Margin": 3,
                "P/FCF": 30,
                "Forward P/E": 30,
                "P/S": 30,
                "Debt/Equity": 3,
            },
        ]
    )

    scored = calculator.add_score(data, ["Ticker", "Fundamental Score"])
    score_by_ticker = scored.set_index("Ticker")["Fundamental Score"].to_dict()

    assert score_by_ticker["WEAK_CORE"] == 75.0
    assert score_by_ticker["STRONG_CORE"] > 75.0


def test_fundamental_score_calculator_curves_final_score():
    calculator = FundamentalScoreCalculator()
    data = pd.DataFrame(
        [
            {
                "Ticker": "LOW",
                "ROIC": 10,
                "EPS Past 5Y": 10,
                "PEG": 3,
            },
            {
                "Ticker": "MID",
                "ROIC": 20,
                "EPS Past 5Y": 20,
                "PEG": 2,
            },
            {
                "Ticker": "HIGH",
                "ROIC": 30,
                "EPS Past 5Y": 30,
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
