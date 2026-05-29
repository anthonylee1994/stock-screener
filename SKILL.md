# Stock Screener API

## 概覽

Base URL: `https://stock-screener.on99.app`

所有請求都需要帶 `api_token` 做身份驗證。

---

## 認證

`api_token` 可以放喺：

- Query string: `?api_token=YOUR_API_TOKEN` (GET request)
- JSON body: `{"api_token": "YOUR_API_TOKEN"}` (POST request)

---

## Endpoints

### `POST /auth`

檢查 `api_token` 係咪有效。

**Request Body**

```json
{
  "api_token": "YOUR_API_TOKEN"
}
```

**Response**

```json
{
  "authorized": true
}
```

---

### `GET /screener`

返回篩選後嘅股票列表，每隻股票都有基本面同技術面評分。

**Query Parameters**

| 參數              | 類型     | 預設值        | 說明                                              |
| ----------------- | -------- | ------------- | ------------------------------------------------- |
| `api_token`       | `string` | —             | **必填**。身份驗證 token                          |
| `sector`          | `string` | `All`         | 板塊篩選，傳 `All` 即不篩選                       |
| `market_cap`      | `string` | `+large`      | 市值篩選，見下方市值選項                          |
| `search`          | `string` | `""`          | 按 ticker 或公司名稱搜尋（模糊匹配）              |
| `tickers`         | `string` | —             | 指定 ticker 列表，逗號分隔，例如 `AAPL,MSFT,GOOG` |
| `potential_stock` | `bool`   | `false`       | 只返回 `Potential Stock` 為 true 嘅股票           |
| `order`           | `string` | `total_score` | 排序欄位，見下方排序選項                          |
| `ascend`          | `bool`   | `false`       | `true` 升序，`false` 降序                         |
| `limit`           | `int`    | `100`         | 每頁數量，最小 1，最大 100                        |
| `offset`          | `int`    | `0`           | 分頁偏移量                                        |

**市值選項 (`market_cap`)**

| 值       | 範圍                      |
| -------- | ------------------------- |
| `+mid`   | Market Cap ≥ $2B          |
| `+large` | Market Cap ≥ $10B（預設） |
| `micro`  | $50M – $300M              |
| `small`  | $300M – $2B               |
| `mid`    | $2B – $10B                |
| `large`  | $10B – $200B              |
| `mega`   | Market Cap ≥ $200B        |

**排序選項 (`order`)**

| 值                  | 排序依據        |
| ------------------- | --------------- |
| `total_score`       | 總分（預設）    |
| `fundamental_score` | 基本面分數      |
| `technical_score`   | 技術面分數      |
| `market_cap`        | 市值            |
| `change_percent`    | 當日升跌幅（%） |
| `volume`            | 成交量          |

**Response**

```json
{
  "data": [...],
  "count": 500,
  "limit": 100,
  "offset": 0,
  "has_more": true,
  "next_offset": 100
}
```

| 欄位          | 說明                                               |
| ------------- | -------------------------------------------------- |
| `data`        | 股票記錄陣列                                       |
| `count`       | 符合篩選條件嘅股票總數                             |
| `limit`       | 本次請求嘅每頁數量                                 |
| `offset`      | 本次請求嘅偏移量                                   |
| `has_more`    | 係咪仲有下一頁                                     |
| `next_offset` | 下一頁嘅 `offset`，`has_more` 為 false 時係 `null` |

**單筆股票記錄結構**

```json
{
  "ticker": "AAPL",
  "name": "Apple Inc.",
  "sector": "Technology",
  "market_cap": 3000000000000,
  "price": 195.5,
  "change": 1.23,
  "change_percent": 0.63,
  "volume": 55000000,
  "total_score": 82.4,
  "potential_stock": false,
  "fundamental": { ... },
  "technical": { ... }
}
```

**`fundamental` 物件**

| 欄位                         | 說明                         |
| ---------------------------- | ---------------------------- |
| `market_cap`                 | 市值（美元）                 |
| `forward_pe`                 | 預期市盈率                   |
| `peg`                        | PEG 比率                     |
| `ps`                         | 市銷率                       |
| `pfcf`                       | 市現率（自由現金流）         |
| `eps_past_5y`                | 過去 5 年 EPS 增長率         |
| `sales_past_5y`              | 過去 5 年收入增長率          |
| `roe`                        | 股東權益回報率               |
| `roic`                       | 投入資本回報率               |
| `profit_margin`              | 利潤率                       |
| `debt_equity`                | 負債比率                     |
| `eps_quarter_over_quarter`   | 最近季度 EPS 同比增長率      |
| `sales_quarter_over_quarter` | 最近季度收入同比增長率       |
| `operating_margin`           | 營業利潤率                   |
| `short_interest`             | Short Float                  |
| `high_52w`                   | 距離 52-week high 嘅相對距離 |
| `target_price`               | 分析師目標價                 |
| `potential_stock`            | 潛力股 boolean flag          |
| `market_cap_score`           | 市值百分位分數（0-100）      |
| `forward_pe_score`           | 預期市盈率百分位分數         |
| `peg_score`                  | PEG 百分位分數               |
| `ps_score`                   | 市銷率百分位分數             |
| `pfcf_score`                 | 市現率百分位分數             |
| `eps_past_5y_score`          | EPS 增長百分位分數           |
| `sales_past_5y_score`        | 收入增長百分位分數           |
| `roe_score`                  | ROE 百分位分數               |
| `roic_score`                 | ROIC 百分位分數              |
| `profit_margin_score`        | 利潤率百分位分數             |
| `debt_equity_score`          | 負債比率百分位分數           |
| `fundamental_score`          | 基本面總分（0-100）          |

**`technical` 物件**

| 欄位               | 說明                                 |
| ------------------ | ------------------------------------ |
| `long_term_score`  | 長線技術分（佔 Technical Score 60%） |
| `mid_term_score`   | 中線技術分（佔 Technical Score 30%） |
| `short_term_score` | 短線技術分（佔 Technical Score 10%） |
| `technical_score`  | 技術面總分（0-100）                  |

---

## 評分方式

### Total Score

```
Raw Total Score = (Fundamental Score × 0.60) + (Technical Score × 0.40)
Total Score = Raw Total Score 拉 curve 到 0-100
```

### Fundamental Score

所有基本面指標都係相對排名（股票池內百分位），按以下權重加總：

| 指標分數            | 權重 |
| ------------------- | ---- |
| ROIC Score          | 20%  |
| EPS Past 5Y Score   | 18%  |
| PEG Score           | 17%  |
| ROE Score           | 13%  |
| Sales Past 5Y Score | 8%   |
| P/FCF Score         | 7%   |
| Profit Margin Score | 6%   |
| Forward P/E Score   | 5%   |
| P/S Score           | 3%   |
| Debt/Equity Score   | 3%   |
| Market Cap Score    | 0%   |

核心 guardrail：

- `ROIC`、`EPS Past 5Y`、`PEG` 三個核心指標至少要有 2 個有效數值；少過 2 個，`Fundamental Score` 最高 60。
- `ROIC Score`、`EPS Past 5Y Score`、`PEG Score` 平均至少要 70；低過 70，`Fundamental Score` 最高 75。

### Potential Stock

`Potential Stock` 係獨立 boolean filter，唔會直接加落 `Fundamental Score`。所有百分比欄位用 ratio 表示，例如 `12.5%` 會係 `0.125`。

要同時符合：

| 條件                   | 門檻                               |
| ---------------------- | ---------------------------------- |
| EPS 季度增長           | `EPS Quarter Over Quarter >= 10%`  |
| 收入季度增長           | `Sales Quarter Over Quarter >= 5%` |
| 經營槓桿               | EPS 季度增長 > 收入季度增長        |
| 估值未離地             | `Forward P/E <= 35` 或 `PEG <= 2`  |
| 營業利潤率             | `Operating Margin >= 8%`           |
| short setup            | `Short Interest >= 3%`             |
| 未離 52-week high 太遠 | `52W High >= -25%`，缺值當通過     |

如果相關資料缺失，`Potential Stock` 會當 `false`。API 可以用：

```text
GET /screener?api_token=YOUR_API_TOKEN&potential_stock=true
```

### Technical Score

技術指標分三個時間維度：

```
Long Term Score  = (EMA200Distance Score + ROC125 Score) / 2
Mid Term Score   = (EMA50Distance Score + ROC20 Score) / 2
Short Term Score = (PPO Slope3 Score + RSI14 Score) / 2

Technical Score = (Long Term × 0.60) + (Mid Term × 0.30) + (Short Term × 0.10)
```

---

## 分頁範例

```
# 第一頁
GET /screener?api_token=YOUR_API_TOKEN&limit=50&offset=0

# 第二頁（用 response 入面嘅 next_offset）
GET /screener?api_token=YOUR_API_TOKEN&limit=50&offset=50
```

---

## 完整請求範例

```
# 篩選科技板塊大型股，按基本面分數降序排列
GET /screener?api_token=YOUR_API_TOKEN&sector=Technology&market_cap=large&order=fundamental_score&limit=20

# 搜尋特定股票
GET /screener?api_token=YOUR_API_TOKEN&search=apple

# 指定 ticker 列表
GET /screener?api_token=YOUR_API_TOKEN&tickers=AAPL,MSFT,NVDA,GOOGL

# 篩選潛力股
GET /screener?api_token=YOUR_API_TOKEN&potential_stock=true
```
