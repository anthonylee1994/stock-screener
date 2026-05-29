import pandas as pd

from stock_screener.utils.screener_rules import (
    CHANGE_PERCENT_COLUMN,
    FUNDAMENTAL_SCORE_COLUMN,
    MARKET_CAP_COLUMN,
    POTENTIAL_STOCK_COLUMN,
    TECHNICAL_SCORE_COLUMN,
    TOTAL_SCORE_COLUMN,
    VOLUME_COLUMN,
)


FUNDAMENTAL_FIELDS = {
    "market_cap": MARKET_CAP_COLUMN,
    "forward_pe": "Forward P/E",
    "peg": "PEG",
    "ps": "P/S",
    "pfcf": "P/FCF",
    "eps_past_5y": "EPS Past 5Y",
    "sales_past_5y": "Sales Past 5Y",
    "roe": "ROE",
    "roic": "ROIC",
    "profit_margin": "Profit Margin",
    "debt_equity": "Debt/Equity",
    "eps_quarter_over_quarter": "EPS Quarter Over Quarter",
    "sales_quarter_over_quarter": "Sales Quarter Over Quarter",
    "operating_margin": "Operating Margin",
    "short_interest": "Short Interest",
    "high_52w": "52W High",
    "target_price": "Target Price",
    "potential_stock": POTENTIAL_STOCK_COLUMN,
    "market_cap_score": "Market Cap Score",
    "forward_pe_score": "Forward P/E Score",
    "peg_score": "PEG Score",
    "ps_score": "P/S Score",
    "pfcf_score": "P/FCF Score",
    "eps_past_5y_score": "EPS Past 5Y Score",
    "sales_past_5y_score": "Sales Past 5Y Score",
    "roe_score": "ROE Score",
    "roic_score": "ROIC Score",
    "profit_margin_score": "Profit Margin Score",
    "debt_equity_score": "Debt/Equity Score",
    "fundamental_score": FUNDAMENTAL_SCORE_COLUMN,
}
TECHNICAL_FIELDS = {
    "long_term_score": "Long Term Score",
    "mid_term_score": "Mid Term Score",
    "short_term_score": "Short Term Score",
    "technical_score": TECHNICAL_SCORE_COLUMN,
}


def format_response(
    data: pd.DataFrame,
    total_count: int | None = None,
    limit: int | None = None,
    offset: int = 0,
    has_more: bool = False,
) -> dict:
    next_offset = offset + len(data) if has_more else None
    return {
        "data": [format_record(row) for _, row in data.iterrows()],
        "count": total_count if total_count is not None else len(data),
        "limit": limit if limit is not None else len(data),
        "offset": offset,
        "has_more": has_more,
        "next_offset": next_offset,
    }


def format_record(row: pd.Series) -> dict:
    return {
        "ticker": clean_value(row.get("Ticker")),
        "name": clean_value(row.get("Company")),
        "sector": clean_value(row.get("Sector")),
        "market_cap": clean_value(row.get(MARKET_CAP_COLUMN)),
        "price": clean_value(first_valid_value(row.get("Price"), row.get("Quote Price"))),
        "change": clean_value(calculate_change(row)),
        "change_percent": clean_value(
            first_valid_value(
                row.get(CHANGE_PERCENT_COLUMN),
                row.get("Quote Change Percent"),
            )
        ),
        "volume": clean_value(row.get(VOLUME_COLUMN)),
        "total_score": clean_value(row.get(TOTAL_SCORE_COLUMN)),
        "potential_stock": clean_bool(row.get(POTENTIAL_STOCK_COLUMN)),
        "fundamental": format_fields(row, FUNDAMENTAL_FIELDS),
        "technical": format_fields(row, TECHNICAL_FIELDS),
    }


def format_fields(row: pd.Series, fields: dict[str, str]) -> dict:
    return {
        response_key: clean_value(row.get(column))
        for response_key, column in fields.items()
    }


def clean_value(value):
    if pd.isna(value):
        return None
    return value


def clean_bool(value):
    if pd.isna(value):
        return None
    return bool(value)


def first_valid_value(*values):
    for value in values:
        if not pd.isna(value):
            return value
    return None


def calculate_change(row: pd.Series):
    price = row.get("Price")
    change_percent = row.get(CHANGE_PERCENT_COLUMN)
    if not pd.isna(price) and not pd.isna(change_percent):
        change_ratio = change_percent / 100
        previous_price = price / (1 + change_ratio)
        return price - previous_price

    quote_change = row.get("Quote Change")
    if not pd.isna(quote_change):
        return quote_change

    return None
