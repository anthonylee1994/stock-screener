import os
from stock_screener.services.api.screener_request import ScreenerRequest
from stock_screener.services.api.screener_response_formatter import format_response
from stock_screener.utils.stock_database import stock_database


API_TOKEN = os.environ.get("API_TOKEN")


class ScreenerService:
    def __init__(
        self,
        database=stock_database,
    ):
        self.database = database

    def is_authorized(self, payload: dict) -> bool:
        return payload.get("api_token") == API_TOKEN

    def get_screener_response(self, payload: dict) -> dict:
        request = ScreenerRequest(payload)
        data, total_count = self.database.read_screener_stocks_with_count(
            limit=request.limit + 1,
            sector=request.sector,
            market_cap=request.market_cap,
            search=request.search,
            tickers=request.tickers,
            order=request.order,
            ascend=request.ascend,
            offset=request.offset,
        )
        has_more = len(data) > request.limit
        page_data = data.head(request.limit)
        return format_response(
            page_data,
            total_count=total_count,
            limit=request.limit,
            offset=request.offset,
            has_more=has_more,
        )


screener_service = ScreenerService()
