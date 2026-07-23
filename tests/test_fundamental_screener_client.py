import pandas as pd
from bs4 import BeautifulSoup

from stock_screener.services.fundamental import fundamental_screener_client as client_module
from stock_screener.services.fundamental.finviz_custom_screener import FinvizCustomScreener
from stock_screener.services.fundamental.finviz_ticker import extract_ticker_from_cell
from stock_screener.services.fundamental.fundamental_screener_client import (
    CUSTOM_COLUMNS,
    DEFAULT_ASCEND,
    DEFAULT_ORDER,
    FUNDAMENTAL_FILTERS,
    FundamentalScreenerClient,
)


DOUBLED_TICKER_CELL_HTML = """
<td align="left" data-boxover-company="NVIDIA Corp" data-boxover-ticker="NVDA" height="10">
  <span class="flex items-center gap-1 pl-0.5">
    <a class="company-ticker" href="stock?t=NVDA">
      <img alt="NVDA logo" src="https://logo.finviz.com/NVDA.svg"/>
      <span>N</span>
    </a>
    <a class="tab-link" href="stock?t=NVDA">NVDA</a>
  </span>
</td>
"""

LEGACY_TICKER_CELL_HTML = """
<td align="left" height="10">
  <a class="tab-link" href="stock?t=AAPL">AAPL</a>
</td>
"""

PLAIN_TICKER_CELL_HTML = "<td>MSFT</td>"


class FakeCustom:
    instances = []

    def __init__(self, result):
        self.result = result
        self.filters = None
        self.view_calls = []
        FakeCustom.instances.append(self)

    def set_filter(self, filters_dict: dict):
        self.filters = filters_dict

    def screener_view(self, **kwargs):
        self.view_calls.append(kwargs)
        return self.result


def test_extract_ticker_from_cell_prefers_data_boxover_ticker():
    col = BeautifulSoup(DOUBLED_TICKER_CELL_HTML, "html.parser").td
    assert col.get_text(strip=True) == "NNVDA"
    assert extract_ticker_from_cell(col) == "NVDA"


def test_extract_ticker_from_cell_falls_back_to_tab_link():
    col = BeautifulSoup(LEGACY_TICKER_CELL_HTML, "html.parser").td
    assert extract_ticker_from_cell(col) == "AAPL"


def test_extract_ticker_from_cell_falls_back_to_plain_text():
    col = BeautifulSoup(PLAIN_TICKER_CELL_HTML, "html.parser").td
    assert extract_ticker_from_cell(col) == "MSFT"


def test_finviz_custom_screener_parses_ticker_without_logo_letter():
    html = f"""
    <table class="screener_table">
      <tr><th>No.</th><th>Ticker</th><th>Company</th></tr>
      <tr>
        <td>1</td>
        {DOUBLED_TICKER_CELL_HTML}
        <td>NVIDIA Corp</td>
      </tr>
    </table>
    """
    soup = BeautifulSoup(html, "html.parser")
    rows = soup.find("table", class_="screener_table").find_all("tr")
    screener = FinvizCustomScreener()

    result = screener._get_table(
        rows=rows,
        df=pd.DataFrame(),
        num_col_index=[],
        table_header=["Ticker", "Company"],
    )

    assert result.to_dict(orient="records") == [
        {"Ticker": "NVDA", "Company": "NVIDIA Corp"}
    ]


def test_fundamental_screener_client_fetch_configures_finviz_custom(monkeypatch):
    FakeCustom.instances = []
    result = pd.DataFrame([{"Ticker": "AAPL"}])

    def custom_factory():
        return FakeCustom(result)

    monkeypatch.setattr(client_module, "FinvizCustomScreener", custom_factory)
    client = FundamentalScreenerClient()

    fetched = client.fetch(limit=50)

    custom = FakeCustom.instances[-1]
    assert fetched.equals(result)
    assert custom.filters == FUNDAMENTAL_FILTERS
    assert custom.filters is not FUNDAMENTAL_FILTERS
    assert custom.view_calls == [
        {
            "order": DEFAULT_ORDER,
            "limit": 50,
            "verbose": 1,
            "ascend": DEFAULT_ASCEND,
            "columns": CUSTOM_COLUMNS,
            "sleep_sec": 0.2,
        }
    ]


def test_fundamental_screener_client_fetch_returns_empty_frame_for_none(monkeypatch):
    FakeCustom.instances = []

    def custom_factory():
        return FakeCustom(None)

    monkeypatch.setattr(client_module, "FinvizCustomScreener", custom_factory)
    client = FundamentalScreenerClient()

    result = client.fetch(limit=10)

    assert result.empty


def test_fundamental_screener_client_requests_potential_stock_columns():
    assert 23 in CUSTOM_COLUMNS
    assert 30 in CUSTOM_COLUMNS
    assert 39 in CUSTOM_COLUMNS
    assert 40 in CUSTOM_COLUMNS
    assert 54 in CUSTOM_COLUMNS
    assert 57 in CUSTOM_COLUMNS
    assert 69 in CUSTOM_COLUMNS
