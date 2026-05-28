from datetime import datetime, timedelta
import logging
import threading
import time
from zoneinfo import ZoneInfo

from stock_screener.services.integrated.integrated_screener_builder import IntegratedScreenerBuilder
from stock_screener.utils.stock_database import stock_database


DEFAULT_LIMIT = 10000
REFRESH_TIMEZONE = ZoneInfo("Asia/Hong_Kong")
REFRESH_HOUR = 8
REFRESH_MINUTE = 15
logger = logging.getLogger(__name__)


class StockRefreshWorker:
    def __init__(
        self,
        database=stock_database,
        integrated_builder: IntegratedScreenerBuilder | None = None,
    ):
        self.database = database
        self.integrated_builder = integrated_builder or IntegratedScreenerBuilder()
        self.started = False

    def refresh_stocks_table(self, force_refresh: bool = False) -> None:
        started_at = time.monotonic()
        logger.info(
            "開始刷新 stocks table force_refresh=%s limit=%s",
            force_refresh,
            DEFAULT_LIMIT,
        )
        try:
            if not force_refresh and self.database.has_stocks():
                data = self.database.read_stocks()
                logger.info(
                    "stocks table 已有資料；server 啟動唔會重新獲取 stocks 或 quote rows=%s elapsed=%.2fs",
                    len(data),
                    time.monotonic() - started_at,
                )
                return

            data = self.integrated_builder.build(
                limit=DEFAULT_LIMIT,
                force_refresh=force_refresh,
            )
            if not data.empty and "Ticker" in data.columns:
                data = self.database.replace_scored_stocks(data)
            logger.info(
                "完成刷新 stocks table force_refresh=%s rows=%s elapsed=%.2fs",
                force_refresh,
                len(data),
                time.monotonic() - started_at,
            )
        except Exception as error:
            logger.exception(
                "刷新 stocks table 失敗 force_refresh=%s elapsed=%.2fs: %s",
                force_refresh,
                time.monotonic() - started_at,
                error,
            )

    def seconds_until_next_refresh(self) -> float:
        now = datetime.now(REFRESH_TIMEZONE)
        next_refresh = datetime.combine(
            now.date(),
            datetime.min.time().replace(hour=REFRESH_HOUR, minute=REFRESH_MINUTE),
            tzinfo=REFRESH_TIMEZONE,
        )
        if next_refresh <= now:
            next_refresh += timedelta(days=1)
        return (next_refresh - now).total_seconds()

    def run(self) -> None:
        logger.info("stocks table 刷新 worker 已啟動")
        self.database.initialize()
        self.refresh_stocks_table(force_refresh=False)
        while True:
            sleep_seconds = self.seconds_until_next_refresh()
            logger.info(
                "下一次 stocks table 刷新會喺 %.0fs 後，%02d:%02d %s 執行",
                sleep_seconds,
                REFRESH_HOUR,
                REFRESH_MINUTE,
                REFRESH_TIMEZONE.key,
            )
            time.sleep(sleep_seconds)
            self.refresh_stocks_table(force_refresh=True)

    def start(self) -> None:
        if self.started:
            return

        self.started = True
        logger.info("啟動 stocks table 刷新 worker thread")
        thread = threading.Thread(
            target=self.run,
            daemon=True,
            name="stocks-table-refresh",
        )
        thread.start()


stock_refresh_worker = StockRefreshWorker()
