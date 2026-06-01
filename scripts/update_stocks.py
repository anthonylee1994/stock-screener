import argparse
import logging
import sys
import time

from stock_screener.services.integrated.integrated_screener_builder import IntegratedScreenerBuilder
from stock_screener.utils.stock_database import stock_database


DEFAULT_LIMIT = 10000
logger = logging.getLogger(__name__)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Manually refresh all persisted stock screener data."
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=DEFAULT_LIMIT,
        help=f"Maximum Finviz screener rows to pull. Default: {DEFAULT_LIMIT}.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    started_at = time.monotonic()

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s [%(name)s] %(message)s",
    )

    logger.info(
        "開始手動更新 stocks table limit=%s db=%s",
        args.limit,
        stock_database.display_name,
    )
    stock_database.initialize()

    data = IntegratedScreenerBuilder().build(limit=args.limit)
    if data.empty or "Ticker" not in data.columns:
        logger.error(
            "手動更新失敗：獲取返嚟嘅資料冇 ticker rows=%s elapsed=%.2fs",
            len(data),
            time.monotonic() - started_at,
        )
        return 1

    filtered_data = stock_database.replace_scored_stocks(data)
    if filtered_data.empty or "Ticker" not in filtered_data.columns:
        logger.error(
            "手動更新失敗：過濾冇 Total Score stocks 後冇 ticker rows=%s elapsed=%.2fs",
            len(filtered_data),
            time.monotonic() - started_at,
        )
        return 1

    logger.info(
        "手動更新完成 rows=%s elapsed=%.2fs",
        len(filtered_data),
        time.monotonic() - started_at,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
