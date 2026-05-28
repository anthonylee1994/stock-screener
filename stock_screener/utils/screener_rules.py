MARKET_CAP_COLUMN = "Market Cap"
TOTAL_SCORE_COLUMN = "Total Score"
FUNDAMENTAL_SCORE_COLUMN = "Fundamental Score"
TECHNICAL_SCORE_COLUMN = "Technical Score"
CHANGE_PERCENT_COLUMN = "Change"
VOLUME_COLUMN = "Volume"
MIN_VOLUME = 1_000_000

MARKET_CAP_RANGES = {
    "+Mid": (2_000_000_000, None),
    "+Large": (10_000_000_000, None),
    "Micro": (50_000_000, 300_000_000),
    "Small": (300_000_000, 2_000_000_000),
    "Mid": (2_000_000_000, 10_000_000_000),
    "Large": (10_000_000_000, 200_000_000_000),
    "Mega": (200_000_000_000, None),
}
MARKET_CAP_BY_API_VALUE = {
    "+mid": "+Mid",
    "+large": "+Large",
    "micro": "Micro",
    "small": "Small",
    "mid": "Mid",
    "large": "Large",
    "mega": "Mega",
}
SORT_COLUMN_BY_VALUE = {
    "market_cap": MARKET_CAP_COLUMN,
    "fundamental_score": FUNDAMENTAL_SCORE_COLUMN,
    "technical_score": TECHNICAL_SCORE_COLUMN,
    "total_score": TOTAL_SCORE_COLUMN,
    "change_percent": CHANGE_PERCENT_COLUMN,
    "volume": VOLUME_COLUMN,
}
SEARCH_COLUMNS = ("Ticker", "Company")


def normalize_market_cap_value(value: str) -> str:
    return MARKET_CAP_BY_API_VALUE.get(str(value).lower(), value)


def normalize_sort_value(value: str) -> str:
    return SORT_COLUMN_BY_VALUE.get(str(value).lower(), value)
