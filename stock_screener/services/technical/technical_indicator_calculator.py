import pandas as pd

from stock_screener.services.technical.technical_price_client import TechnicalPriceClient


class TechnicalIndicatorCalculator:
    def __init__(self, price_client: TechnicalPriceClient | None = None):
        self.price_client = price_client or TechnicalPriceClient()

    def calculate_indicators(
        self,
        price_data: pd.DataFrame,
        tickers: list[str],
    ) -> pd.DataFrame:
        close_prices = self.price_client.extract_close_prices(price_data, tickers)
        volumes = self.price_client.extract_field(price_data, tickers, "Volume")
        rows = []
        for ticker in tickers:
            row = self.calculate_ticker_indicators(
                ticker=ticker,
                close_prices=close_prices,
                volumes=volumes,
            )
            if row is not None:
                rows.append(row)

        return pd.DataFrame(rows)

    def calculate_ticker_indicators(
        self,
        ticker: str,
        close_prices: pd.DataFrame,
        volumes: pd.DataFrame,
    ) -> dict | None:
        if ticker not in close_prices.columns:
            return None

        close = close_prices[ticker].dropna()
        if len(close) < 201:
            return None

        latest_close = close.iloc[-1]
        previous_close = close.iloc[-2]
        ema200 = close.ewm(span=200, adjust=False).mean().iloc[-1]
        ema50 = close.ewm(span=50, adjust=False).mean().iloc[-1]
        ppo = self.calculate_ppo(close)
        volume = self.latest_volume(ticker, volumes)

        return {
            "Ticker": ticker,
            "Quote Price": latest_close,
            "Quote Change": latest_close - previous_close,
            "Quote Change Percent": (latest_close - previous_close) / previous_close,
            "Quote Volume": volume,
            "EMA200Distance": (latest_close / ema200) - 1,
            "ROC125": (latest_close / close.iloc[-126]) - 1,
            "EMA50Distance": (latest_close / ema50) - 1,
            "ROC20": (latest_close / close.iloc[-21]) - 1,
            "PPO Slope3": ppo.diff(3).iloc[-1] / 3,
            "RSI14": self.calculate_rsi14(close),
        }

    def latest_volume(self, ticker: str, volumes: pd.DataFrame):
        if ticker not in volumes.columns:
            return None

        volume_series = volumes[ticker].dropna()
        if volume_series.empty:
            return None
        return volume_series.iloc[-1]

    def calculate_ppo(self, close: pd.Series) -> pd.Series:
        ema12 = close.ewm(span=12, adjust=False).mean()
        ema26 = close.ewm(span=26, adjust=False).mean()
        return ((ema12 - ema26) / ema26) * 100

    def calculate_rsi14(self, close: pd.Series) -> float:
        delta = close.diff()
        gain = delta.clip(lower=0)
        loss = -delta.clip(upper=0)
        average_gain = gain.ewm(alpha=1 / 14, adjust=False).mean()
        average_loss = loss.ewm(alpha=1 / 14, adjust=False).mean()
        relative_strength = average_gain / average_loss
        return (100 - (100 / (1 + relative_strength))).iloc[-1]
