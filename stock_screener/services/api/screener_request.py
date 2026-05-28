from stock_screener.utils.screener_rules import (
    MARKET_CAP_BY_API_VALUE as SCREENER_MARKET_CAP_BY_API_VALUE,
    SORT_COLUMN_BY_VALUE as SCREENER_SORT_COLUMN_BY_VALUE,
    normalize_market_cap_value,
    normalize_sort_value,
)
from stock_screener.utils.ticker_normalizer import normalize_tickers


DEFAULT_SECTOR = "All"
DEFAULT_MARKET_CAP = "+large"
DEFAULT_ORDER = "total_score"
DEFAULT_ASCEND = False
DEFAULT_LIMIT = 100
MAX_LIMIT = 100
DEFAULT_OFFSET = 0
MARKET_CAP_BY_API_VALUE = SCREENER_MARKET_CAP_BY_API_VALUE
ORDER_BY_API_VALUE = SCREENER_SORT_COLUMN_BY_VALUE


class ScreenerRequest:
    def __init__(self, payload: dict):
        self.payload = payload

    @property
    def sector(self) -> str:
        return self.payload.get("sector", DEFAULT_SECTOR)

    @property
    def market_cap(self) -> str:
        value = self.payload.get("market_cap", DEFAULT_MARKET_CAP)
        return normalize_market_cap_value(value)

    @property
    def order(self) -> str:
        value = self.payload.get("order", DEFAULT_ORDER)
        return normalize_sort_value(value)

    @property
    def ascend(self) -> bool:
        return self.parse_bool(self.payload.get("ascend"), DEFAULT_ASCEND)

    @property
    def search(self) -> str:
        return self.payload.get("search", "")

    @property
    def tickers(self) -> list[str]:
        return normalize_tickers(self.payload.get("tickers"))

    @property
    def limit(self) -> int:
        return self.parse_int(
            value=self.payload.get("limit"),
            default=DEFAULT_LIMIT,
            minimum=1,
            maximum=MAX_LIMIT,
        )

    @property
    def offset(self) -> int:
        return self.parse_int(
            value=self.payload.get("offset"),
            default=DEFAULT_OFFSET,
            minimum=0,
        )

    def parse_bool(self, value: str | None, default: bool) -> bool:
        if isinstance(value, bool):
            return value
        if value is None:
            return default
        return str(value).lower() in ["1", "true", "yes", "y", "on"]

    def parse_int(
        self,
        value: str | int | None,
        default: int,
        minimum: int,
        maximum: int | None = None,
    ) -> int:
        try:
            parsed_value = int(value)
        except (TypeError, ValueError):
            parsed_value = default

        parsed_value = max(minimum, parsed_value)
        if maximum is not None:
            parsed_value = min(maximum, parsed_value)
        return parsed_value
