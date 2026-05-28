import logging

import pandas as pd
import yfinance as yf


DEFAULT_PERIOD = "1y"
DEFAULT_INTERVAL = "1d"
DOWNLOAD_RETRY_ATTEMPTS = 2
DOWNLOAD_RETRY_CHUNK_SIZE = 25
logger = logging.getLogger(__name__)


class TechnicalPriceClient:
    def download_price_data(self, tickers: list[str]) -> pd.DataFrame:
        price_data = self.download_tickers(tickers)
        failed_tickers = self.find_failed_tickers(price_data, tickers)
        for attempt in range(1, DOWNLOAD_RETRY_ATTEMPTS + 1):
            if not failed_tickers:
                break

            logger.warning(
                "技術面價格下載有 ticker 失敗，開始 retry attempt=%s failed=%s",
                attempt,
                len(failed_tickers),
            )
            retry_data = self.download_tickers_in_chunks(failed_tickers)
            price_data = self.merge_price_data(price_data, retry_data)
            failed_tickers = self.find_failed_tickers(price_data, failed_tickers)

        if failed_tickers:
            logger.warning(
                "技術面價格下載 retry 後仍然失敗 tickers=%s sample=%s",
                len(failed_tickers),
                failed_tickers[:20],
            )

        return price_data

    def download_tickers(self, tickers: list[str]) -> pd.DataFrame:
        try:
            price_data = yf.download(
                tickers=tickers,
                period=DEFAULT_PERIOD,
                interval=DEFAULT_INTERVAL,
                auto_adjust=True,
                progress=True,
                group_by="column",
                threads=True,
            )
        except Exception as error:
            logger.warning(
                "技術面價格下載批次失敗 tickers=%s: %s",
                len(tickers),
                error,
            )
            return pd.DataFrame()

        return self.normalize_price_data(price_data, tickers)

    def download_tickers_in_chunks(self, tickers: list[str]) -> pd.DataFrame:
        chunks = []
        for start in range(0, len(tickers), DOWNLOAD_RETRY_CHUNK_SIZE):
            chunk = tickers[start : start + DOWNLOAD_RETRY_CHUNK_SIZE]
            chunk_data = self.download_tickers(chunk)
            if not chunk_data.empty:
                chunks.append(chunk_data)

        if not chunks:
            return pd.DataFrame()

        return self.merge_price_data(*chunks)

    def normalize_price_data(
        self,
        price_data: pd.DataFrame,
        tickers: list[str],
    ) -> pd.DataFrame:
        if price_data.empty:
            return pd.DataFrame()
        if isinstance(price_data.columns, pd.MultiIndex):
            return price_data
        if len(tickers) == 1:
            normalized_data = price_data.copy()
            normalized_data.columns = pd.MultiIndex.from_product(
                [normalized_data.columns, [tickers[0]]]
            )
            return normalized_data
        return price_data

    def merge_price_data(self, *frames: pd.DataFrame) -> pd.DataFrame:
        valid_frames = [frame for frame in frames if not frame.empty]
        if not valid_frames:
            return pd.DataFrame()

        merged_data = pd.concat(valid_frames, axis=1)
        return merged_data.loc[:, ~merged_data.columns.duplicated(keep="last")]

    def find_failed_tickers(
        self,
        price_data: pd.DataFrame,
        tickers: list[str],
    ) -> list[str]:
        close_prices = self.extract_close_prices(price_data, tickers)
        return [
            ticker
            for ticker in tickers
            if ticker not in close_prices.columns or close_prices[ticker].dropna().empty
        ]

    def extract_close_prices(
        self,
        price_data: pd.DataFrame,
        tickers: list[str],
    ) -> pd.DataFrame:
        return self.extract_field(price_data, tickers, "Close")

    def extract_field(
        self,
        price_data: pd.DataFrame,
        tickers: list[str],
        field: str,
    ) -> pd.DataFrame:
        if price_data.empty:
            return pd.DataFrame()
        if isinstance(price_data.columns, pd.MultiIndex):
            if field not in price_data.columns.get_level_values(0):
                return pd.DataFrame()
            return price_data[field]
        if len(tickers) == 1 and field in price_data.columns:
            return price_data[[field]].rename(columns={field: tickers[0]})
        return pd.DataFrame()
