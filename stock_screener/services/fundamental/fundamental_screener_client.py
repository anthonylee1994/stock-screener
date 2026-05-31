import pandas as pd
from finvizfinance.screener.custom import Custom


FUNDAMENTAL_FILTERS = {
    "Market Cap.": "+Mid (over $2bln)",
    "Average Volume": "Over 1M",
    # "EPS growthpast 5 years": "Over 20%",
    # "Return on Equity": "Over +15%",
}
DEFAULT_ORDER = "Market Cap."
DEFAULT_ASCEND = False
CUSTOM_COLUMNS = [
    1,
    2,
    3,
    6,
    8,
    9,
    10,
    13,
    19,
    21,
    22,
    23,
    30,
    33,
    34,
    39,
    38,
    40,
    41,
    54,
    57,
    69,
    65,
    66,
    67,
]


class FundamentalScreenerClient:
    def fetch(self, limit: int) -> pd.DataFrame:
        screener = Custom()
        screener.set_filter(filters_dict=FUNDAMENTAL_FILTERS.copy())
        result = screener.screener_view(
            order=DEFAULT_ORDER,
            limit=limit,
            verbose=1,
            ascend=DEFAULT_ASCEND,
            columns=CUSTOM_COLUMNS,
            sleep_sec=0.2,
        )
        if result is None:
            return pd.DataFrame()
        return result
