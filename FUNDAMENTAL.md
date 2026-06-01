# 股票基本面分析評分

## 評分點樣計？

### 核心邏輯：基本面相對排名

系統唔係用固定門檻話「ROE 一定要大過 15%」先合格，而係將每隻股票同同板塊股票比較，計出 0-100 分嘅百分位排名。如果某個板塊某個指標有效樣本少過 5 隻，就 fallback 用全股票池排名。

流程：

1. 先由 fundamental screener 攞基本面資料
2. 清走冇 ticker 嘅無效資料
3. 將 market cap、估值、增長、回報率、負債等欄位轉成數字
4. 每個可評分欄位優先喺同 `Sector` 入面做百分位排名；板塊樣本不足先用全股票池排名
5. 按權重加總成 `Fundamental Score`
6. 最後按 `Fundamental Score` 由高至低排序

**例子：**

> 如果某隻科技股嘅 PEG 喺科技股入面最低，PEG Score 會接近 100；如果 ROE 喺同板塊最高，ROE Score 亦會接近 100。總分就係將呢啲分數按權重加埋。

---

## 指標方向

有啲指標越大越好，有啲就越細越好。

| 指標            | 方向     | 解讀                                           |
| --------------- | -------- | ---------------------------------------------- |
| `Market Cap`    | 越大越好 | 大市值通常代表規模、流動性同抗風險能力較高     |
| `EPS Past 5Y`   | 越大越好 | 過去 5 年每股盈利增長越高越好                  |
| `Sales Past 5Y` | 越大越好 | 過去 5 年收入增長越高越好                      |
| `ROE`           | 越大越好 | 股東權益回報率越高，代表用股東資本賺錢效率越好 |
| `ROIC`          | 越大越好 | 投入資本回報率越高，代表資本配置效率越好       |
| `Profit Margin` | 越大越好 | 利潤率越高，通常代表定價能力或成本控制較好     |
| `Forward P/E`   | 越細越好 | 預期市盈率越低，估值越平                       |
| `PEG`           | 越細越好 | 增長調整後估值越低越好                         |
| `P/S`           | 越細越好 | 市銷率越低，收入相對估值越平                   |
| `P/FCF`         | 越細越好 | 自由現金流估值越低越好                         |
| `Debt/Equity`   | 越細越好 | 負債相對股本越低，財務槓桿風險越低             |

---

## 額外 Finviz 欄位

除咗評分用嘅核心基本面欄位，系統亦會由 Finviz Custom Screener 攞以下欄位，主要用嚟顯示同判斷 `Potential Stock`：

| 系統欄位                     | Finviz 欄位    | 解讀                               |
| ---------------------------- | -------------- | ---------------------------------- |
| `EPS Quarter Over Quarter`   | `EPS Q/Q`      | 最近季度 EPS 同比增長              |
| `Sales Quarter Over Quarter` | `Sales Q/Q`    | 最近季度收入同比增長               |
| `Gross Margin`               | `Gross M`      | 毛利率                             |
| `Operating Margin`           | `Oper M`       | 營業利潤率，反映營運槓桿同成本控制 |
| `Short Interest`             | `Short Float`  | short float，顯示用資料            |
| `52W High`                   | `52W High`     | 股價距離 52-week high 嘅相對距離   |
| `Target Price`               | `Target Price` | 分析師目標價                       |
| `Target Price Upside` | 計算欄位 | `(Target Price - Price) / Price`，可用 `target_price_upside` 排序 |

百分比欄位會統一用 ratio 儲存，例如 `12.5%` 會儲成 `0.125`。如果 Finvizfinance 已經回傳 `0.125` 呢類 ratio，系統會直接保留，唔會再除 100。

---

## 總分權重

現時基本面權重會覆蓋增長、質素、估值、規模同負債風險：

| 指標分數              | 比重    |
| --------------------- | ------- |
| `ROIC Score`          | **20%** |
| `EPS Past 5Y Score`   | **18%** |
| `PEG Score`           | **17%** |
| `ROE Score`           | **13%** |
| `Sales Past 5Y Score` | **8%**  |
| `P/FCF Score`         | **7%**  |
| `Profit Margin Score` | **6%**  |
| `Forward P/E Score`   | **5%**  |
| `P/S Score`           | **3%**  |
| `Debt/Equity Score`   | **3%**  |
| `Market Cap Score`    | **0%**  |

公式：

```text
Fundamental Score =
  (ROIC Score * 0.20)
+ (EPS Past 5Y Score * 0.18)
+ (PEG Score * 0.17)
+ (ROE Score * 0.13)
+ (Sales Past 5Y Score * 0.08)
+ (P/FCF Score * 0.07)
+ (Profit Margin Score * 0.06)
+ (Forward P/E Score * 0.05)
+ (P/S Score * 0.03)
+ (Debt/Equity Score * 0.03)
+ (Market Cap Score * 0.00)
```

即係目前評分最重視：

1. 盈利同收入增長
2. ROIC / ROE / Profit Margin 呢類營運質素
3. PEG、P/FCF、Forward P/E 呢類估值
4. 負債風險；市值只顯示排名分數，唔再直接推高基本面總分

另外有兩條核心資料 guardrail：

- `ROIC`、`EPS Past 5Y`、`PEG` 三個核心指標入面，至少要有 2 個有效數值；如果少過 2 個，`Fundamental Score` 最高只會係 60 分。
- `ROIC Score`、`EPS Past 5Y Score`、`PEG Score` 三個核心分數平均要至少 70；如果低過 70，`Fundamental Score` 最高只會係 75 分。

---

## Potential Stock 點計？

`Potential Stock` 係 boolean filter，唔係 `Fundamental Score` 嘅一部分。佢用嚟搵「市場預期可能太低，但近期基本面開始轉強」嘅股票。

現時符合以下完整條件先會標記為 `Potential Stock`：

```text
(Market Cap >= 2B)
AND
(EPS Past 5Y > 15% OR Sales Past 5Y > 15%)
AND
(Profit Margin > 0)
AND
(PEG < 1)
AND
(ROE > 15%)
AND
(Volume >= 500K)
AND
(Price above SMA200)
```

| 條件     | 門檻                                         |
| -------- | -------------------------------------------- |
| 市值     | `Market Cap >= 2B`                           |
| 成長     | `EPS Past 5Y > 15%` 或 `Sales Past 5Y > 15%` |
| 盈利能力 | `Profit Margin > 0`                          |
| 估值     | `PEG < 1`                                    |
| 營運質素 | `ROE > 15%`                                  |
| 流動性   | `Volume >= 500K`                             |
| 趨勢     | `200-Day Simple Moving Average > 0`，等同 `Price above SMA200` |

如果相關資料缺失，`Potential Stock` 會當 `False`，唔會用 `NULL`。系統內部會將百分比欄位儲存成 ratio，例如 `15%` 係 `0.15`。API 可以用 `potential_stock=true` 篩走非潛力股。

呢個 filter 係 Finviz 資料做到嘅 proxy，唔等於真正 analyst revision model。佢冇直接睇 30-90 日 EPS estimate revision、訂單 backlog、指引同 SEC filing；中咗 filter 只代表值得再深挖。

---

## 分數點解讀？

| 分數範圍   | 意思                                                           |
| ---------- | -------------------------------------------------------------- |
| **90-100** | 股票池入面基本面排名最前，增長、估值、回報率或者市值有明顯優勢 |
| **70-89**  | 基本面偏強，可以優先研究                                       |
| **50-69**  | 中等，可能某啲指標好、某啲指標拖後腿                           |
| **30-49**  | 偏弱，喺股票池入面唔突出                                       |
| **0-29**   | 基本面相對落後，或者關鍵資料缺失                               |

**注意：** 呢個分數係相對排名。100 分只代表佢喺今次股票池入面排得最高，唔代表絕對便宜、絕對安全，或者一定值得買。

---

## 同 Total Score 嘅關係

完整 screener 會再將基本面分數同技術面分數合成 `Total Score`：

```text
Raw Total Score = (Fundamental Score * 0.60) + (Technical Score * 0.40)
Total Score = 將 Raw Total Score 拉 curve 到 0-100
```

如果冇技術面分數，系統會保留基本面資料，但 `Total Score` 會係空值。

---

## 使用注意事項

1. **板塊同股票池都好重要**：分數優先係板塊內相對排名；如果板塊樣本不足，先會退回全股票池排名。同一隻股票喺唔同篩選條件下，排名可以差好遠。
2. **缺失資料會輸蝕**：某個指標冇數據會計 0 分，資料唔齊嘅股票總分可能偏低。
3. **估值平唔等於抵買**：PEG、P/E、P/S 低可能係市場低估，亦可能係公司前景轉差。
4. **增長係歷史數據**：`EPS Past 5Y` 同 `Sales Past 5Y` 反映過去，唔保證未來繼續增長。
5. **唔係買賣訊號**：高分只係話基本面喺股票池入面相對靚，入場仍然要配合價格、風險、倉位同自己策略。
