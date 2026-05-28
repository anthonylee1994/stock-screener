def normalize_tickers(tickers) -> list[str]:
    normalized_tickers = []
    seen_tickers = set()
    for ticker in iter_tickers(tickers):
        normalized_ticker = str(ticker).strip().upper()
        if not normalized_ticker or normalized_ticker in seen_tickers:
            continue
        normalized_tickers.append(normalized_ticker)
        seen_tickers.add(normalized_ticker)
    return normalized_tickers


def iter_tickers(tickers):
    if tickers is None:
        return []
    if isinstance(tickers, str):
        return tickers.split(",")
    return tickers
