# Stock Screener API

Flask API for an integrated US stock screener. It combines Finviz fundamental
screening and quote data, Yahoo Finance technical indicators, and a weighted
total score.

## Features

- Pulls the first 1000 screened stocks from Finviz fundamentals.
- Calculates fundamental percentile scores for valuation, growth, return, and
  market cap metrics.
- Stores extra Finviz operating, short-interest, 52-week-high, and target-price
  fields for potential-stock screening.
- Exposes a `potential_stock` boolean filter for stocks with recent growth,
  operating leverage, reasonable valuation, and short-interest setup.
- Calculates technical scores from long-, mid-, and short-term momentum
  indicators.
- Stores the full screener result in SQLite so API
  requests do not trigger a full refresh.
- Refreshes the stocks table in a background worker at 08:15 Asia/Hong_Kong
  time.
- Stores Finviz quote fields with the screener result.
- Applies sector, market-cap, search, sorting, and response limiting on the
  database side.

## Requirements

- Python 3.14+
- `uv`

## Project Structure

```text
stock_screener/
  app.py                 Flask app factory and Gunicorn entrypoint
  controllers/           HTTP route handlers
  services/
    api/                 API request/response orchestration
    common/              Shared scoring and normalization helpers
    fundamental/         Finviz fundamentals and fundamental scoring
    integrated/          Combined fundamental/technical screener builder
    jobs/                Background refresh worker
    technical/           Yahoo Finance prices and technical scoring
  utils/                 SQLite schema, queries, and persistence helpers
scripts/
  update_stocks.py   Manual refresh command
data/
  db.sqlite          Local SQLite database (ignored by git)
```

## Run Locally

```bash
uv sync
uv run python -m stock_screener.app
```

The server listens on:

```text
http://localhost:3000
```

On startup, the app initializes the configured SQLite database. If the
`stocks` table already has rows, the server uses those rows without refreshing
quote data. If the table is empty, it builds the full integrated screener list
before serving useful results. At 08:15 Asia/Hong_Kong, the background worker
force-refreshes the stocks table and stores everything back into the database.

## Configuration

| Variable         | Default            | Description                      |
| ---------------- | ------------------ | -------------------------------- |
| `PORT`           | `3000`             | Port used by Gunicorn in Dokku.  |
| `SQLITE_DB_PATH` | `./data/db.sqlite` | SQLite database path for stocks. |

The API token is configured in
`stock_screener/services/api/screener_service.py`.

## API

### `GET /screener`

Returns paginated integrated screener rows as JSON. The service applies
filtering, sorting, limiting, and offsetting in SQL against the persisted stock
universe.

Requests can pass fields as query parameters. `get_request_payload()` also
merges a JSON body if one is provided, but the route is exposed as `GET`.

### Query Parameters

| Field        | Type    | Default       | Description                                                                                                                        |
| ------------ | ------- | ------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `api_token`  | string  | required      | Must match the configured API token.                                                                                               |
| `sector`     | string  | `All`         | Sector filter. Use `All` for no sector filter.                                                                                     |
| `market_cap` | string  | `+large`      | Market-cap filter. Supported API values: `+mid`, `+large`, `mid`, `large`, `mega`.                                                 |
| `search`     | string  | empty         | Case-insensitive substring search against ticker and company/name.                                                                 |
| `potential_stock` | boolean | `false` | When true, only returns rows flagged as potential stocks. Accepted true values include `1`, `true`, `yes`, `y`, and `on`. |
| `order`      | string  | `total_score` | Sort field. Supported API values: `market_cap`, `fundamental_score`, `technical_score`, `total_score`, `change_percent`, `volume`. |
| `ascend`     | boolean | `false`       | Sort ascending when true. Accepted true values include `1`, `true`, `yes`, `y`, and `on`. Anything else is treated false.          |
| `limit`      | integer | `100`         | Number of rows to return for this page. Values below `1` become `1`; values above `100` become `100`.                              |
| `offset`     | integer | `0`           | Number of matching rows to skip before returning this page. Values below `0` become `0`.                                           |

### `POST /auth`

Checks whether the supplied API token is valid.

Request:

```bash
curl -X POST "http://localhost:3000/auth" \
  -H "Content-Type: application/json" \
  -d '{"api_token":"YOUR_API_TOKEN"}'
```

Response:

```json
{
  "authorized": true
}
```

## Examples

Default screener:

```bash
curl "http://localhost:3000/screener?api_token=YOUR_API_TOKEN"
```

Technology large-cap stocks sorted by volume:

```bash
curl "http://localhost:3000/screener?api_token=YOUR_API_TOKEN&sector=Technology&market_cap=large&order=volume"
```

Sort by daily change percent:

```bash
curl "http://localhost:3000/screener?api_token=YOUR_API_TOKEN&order=change_percent"
```

Search by ticker or company name:

```bash
curl "http://localhost:3000/screener?api_token=YOUR_API_TOKEN&search=nvidia&order=total_score"
```

Filter for potential stocks:

```bash
curl "http://localhost:3000/screener?api_token=YOUR_API_TOKEN&potential_stock=true"
```

Sort by total score ascending:

```bash
curl "http://localhost:3000/screener?api_token=YOUR_API_TOKEN&order=total_score&ascend=true"
```

Load the next page for scroll-and-load-more:

```bash
curl "http://localhost:3000/screener?api_token=YOUR_API_TOKEN&limit=50&offset=50"
```

Frontend load-more flow:

- First request: call `/screener` with `limit`, no `offset`, and render `data`.
- Next request: when the list scrolls near the bottom and `has_more` is true,
  call the same filters with `offset=next_offset`.
- Append each returned `data` page to the existing list.
- When `sector`, `market_cap`, `search`, `order`, or `ascend` changes, clear the
  list and start again from `offset=0`.
- Stop requesting more rows when `has_more` is false.

## Response

`count` is the total number of records matching the filter criteria, not the
number of records in the current page.

Success:

```json
{
  "count": 100,
  "limit": 100,
  "offset": 0,
  "has_more": true,
  "next_offset": 100,
  "data": [
    {
      "ticker": "NVDA",
      "name": "NVIDIA Corp",
      "sector": "Technology",
      "market_cap": 5475280000000.0,
      "price": 225.320007,
      "change": -10.419998,
      "change_percent": -4.4201,
      "volume": 179993300,
      "total_score": 80.72,
      "potential_stock": false,
      "fundamental": {
        "market_cap": 5475280000000.0,
        "forward_pe": 19.95,
        "peg": 0.5,
        "ps": 20.45,
        "pfcf": 28.45,
        "eps_past_5y": 0.9527,
        "sales_past_5y": 0.669,
        "roe": 1.0149,
        "roic": 0.7482,
        "profit_margin": 0.6297,
        "debt_equity": 0.07,
        "eps_quarter_over_quarter": 2.1342,
        "sales_quarter_over_quarter": 0.8523,
        "operating_margin": 0.6402,
        "short_interest": 0.0128,
        "high_52w": -0.0942,
        "target_price": 306.8,
        "potential_stock": false,
        "market_cap_score": 100.0,
        "forward_pe_score": 59.18,
        "peg_score": 95.92,
        "ps_score": 31.63,
        "pfcf_score": 72.45,
        "eps_past_5y_score": 91.84,
        "sales_past_5y_score": 95.92,
        "roe_score": 93.88,
        "roic_score": 91.84,
        "profit_margin_score": 93.88,
        "debt_equity_score": 83.67,
        "fundamental_score": 88.16
      },
      "technical": {
        "long_term_score": 64.29,
        "mid_term_score": 84.69,
        "short_term_score": 92.86,
        "technical_score": 73.27
      }
    }
  ]
}
```

Unauthorized:

```json
{
  "error": "Unauthorized"
}
```

HTTP status: `401`

## Scoring

Fundamental metric scores are percentile scores from `0` to `100` within the
screened stock pool:

- Higher is better: `Market Cap`, `EPS Past 5Y`, `Sales Past 5Y`, `ROE`,
  `ROIC`, `Profit Margin`
- Lower is better: `Forward P/E`, `PEG`, `P/S`, `P/FCF`, `Debt/Equity`

`Fundamental Score` uses percentage weights from `SCORE_WEIGHTS` in
`stock_screener/services/fundamental/fundamental_score_calculator.py`.

`Potential Stock` is a separate boolean flag, not part of `Fundamental Score`.
It is true only when the row has:

- `EPS Quarter Over Quarter >= 10%`
- `Sales Quarter Over Quarter >= 5%`
- EPS quarter-over-quarter growth greater than sales quarter-over-quarter growth
- `Forward P/E <= 35` or `PEG <= 2`
- `Operating Margin >= 8%`
- `Short Interest >= 3%`
- `52W High >= -25%`; missing `52W High` is allowed

Missing required potential-stock inputs are treated as false. Percent-like
Finviz fields are stored as ratios, so `12.5%` is returned as `0.125`.
More detail is in `FUNDAMENTAL.md`.

Technical scoring follows `TECHNICAL.md`:

```text
Technical Score = Long Term Score * 0.6 + Mid Term Score * 0.3 + Short Term Score * 0.1
```

Integrated scoring:

```text
Raw Total Score = Fundamental Score * 0.6 + Technical Score * 0.4
Total Score = min-max curve Raw Total Score to 0-100
```

Rows without a total score are filtered out before response sorting.

## Data Flow

- Full stock universe: persisted in local SQLite table `stocks`, including quote
  columns.
- API requests only read the persisted `stocks` table.
- API filtering, sorting, and limiting are handled in SQL.
- API requests do not use in-memory screener or quote caches.
- API requests do not trigger a Finviz, technical, or quote refresh.
- Startup only rebuilds the table when `stocks` is empty.
- The daily 08:15 Asia/Hong_Kong worker rebuilds the table and writes fresh
  Finviz quote fields into SQLite.

## Data Sources

- Fundamentals and initial screen: Finviz via `finvizfinance`
- Potential-stock fields: Finviz Custom Screener fields `EPS Q/Q`, `Sales Q/Q`,
  `Oper M`, `Short Float`, `52W High`, and `Target Price`
- Top-level quote fields (`price`, `change`, `change_percent`, `volume`):
  Finviz via `finvizfinance`
- Technical price history: Yahoo Finance via `yfinance`

## Dokku

The app uses the `Procfile` web process and stores SQLite at
`./data/db.sqlite` by default. Mount persistent storage to `/app/data`:

```bash
dokku apps:create stock-screener
dokku storage:ensure-directory stock-screener
dokku storage:mount stock-screener /var/lib/dokku/data/storage/stock-screener:/app/data
git push dokku main
```

Optional manual refresh:

```bash
dokku run stock-screener uv run python -m scripts.update_stocks
```

## Docker

Build:

```bash
docker build -t stock-screener .
```

Run:

```bash
docker run --rm -p 3000:3000 -v "$PWD/data:/app/data" stock-screener
```

The container starts Gunicorn on `0.0.0.0:${PORT:-3000}`.
