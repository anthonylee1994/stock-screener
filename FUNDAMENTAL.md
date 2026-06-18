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

> 如果某隻科技股嘅 Gross Margin 喺科技股入面最高，Gross Margin Score 會接近 100；如果 ROE 喺同板塊最高，ROE Score 亦會接近 100。總分就係將呢啲分數按權重加埋。

---

## 指標方向

有啲指標越大越好，有啲就越細越好。

| 指標            | 方向     | 解讀                                                       |
| --------------- | -------- | ---------------------------------------------------------- |
| `Market Cap`    | 越大越好 | 大市值通常代表規模、流動性同抗風險能力較高                 |
| `ROE`           | 越大越好 | 股東權益回報率越高，代表用股東資本賺錢效率越好（護城河）   |
| `Gross Margin`  | 越大越好 | 毛利率越高，通常代表定價能力越強（護城河 proxy）           |
| `EPS Past 5Y`   | 越大越好 | 過去 5 年每股盈利增長越高越好                              |
| `Sales Past 5Y` | 越大越好 | 過去 5 年收入增長越高越好                                  |
| `Debt/Equity`   | 越細越好 | 負債相對股本越低，財務槓桿風險越低                         |

`ROIC`、`Profit Margin`、`Forward P/E`、`PEG`、`P/S`、`P/FCF` 仍然會由 Finviz 攞嚟做顯示，但已經唔再納入 `Fundamental Score` 評分。

---

## 額外 Finviz 欄位

除咗評分用嘅核心基本面欄位，系統亦會由 Finviz Custom Screener 攞以下欄位，主要用嚟顯示同判斷 `Potential Stock`：

| 系統欄位                     | Finviz 欄位    | 解讀                                                              |
| ---------------------------- | -------------- | ----------------------------------------------------------------- |
| `EPS Quarter Over Quarter`   | `EPS Q/Q`      | 最近季度 EPS 同比增長                                             |
| `Sales Quarter Over Quarter` | `Sales Q/Q`    | 最近季度收入同比增長                                              |
| `Gross Margin`               | `Gross M`      | 毛利率                                                            |
| `Operating Margin`           | `Oper M`       | 營業利潤率，反映營運槓桿同成本控制                                |
| `Short Interest`             | `Short Float`  | short float，顯示用資料                                           |
| `52W High`                   | `52W High`     | 股價距離 52-week high 嘅相對距離                                  |
| `Target Price`               | `Target Price` | 分析師目標價                                                      |
| `Target Price Upside`        | 計算欄位       | `(Target Price - Price) / Price`，可用 `target_price_upside` 排序 |

百分比欄位會統一用 ratio 儲存，例如 `12.5%` 會儲成 `0.125`。如果 Finvizfinance 已經回傳 `0.125` 呢類 ratio，系統會直接保留，唔會再除 100。

---

## 總分權重

現時基本面評分對齊 Potential Stock 嘅「護城河 + 增長 + 穩健」，淨係評分以下 6 個指標（`Market Cap` 淨係計排名分數，weight 0，唔推高總分）：

| 指標分數              | 比重    |
| --------------------- | ------- |
| `ROE Score`           | **25%** |
| `EPS Past 5Y Score`   | **25%** |
| `Gross Margin Score`  | **20%** |
| `Sales Past 5Y Score` | **15%** |
| `Debt/Equity Score`   | **15%** |
| `Market Cap Score`    | **0%**  |

公式：

```text
Fundamental Score =
  (ROE Score * 0.25)
+ (EPS Past 5Y Score * 0.25)
+ (Gross Margin Score * 0.20)
+ (Sales Past 5Y Score * 0.15)
+ (Debt/Equity Score * 0.15)
+ (Market Cap Score * 0.00)
```

即係目前評分最重視：

1. 護城河 —— `ROE`（資本回報）+ `Gross Margin`（定價力）
2. 增長 —— `EPS Past 5Y` + `Sales Past 5Y`
3. 穩健 —— `Debt/Equity`；市值只顯示排名分數，唔直接推高總分

另外有幾條 guardrail：

- `ROE`、`Gross Margin`、`EPS Past 5Y` 三個核心指標入面，至少要有 2 個有效數值；如果少過 2 個，`Fundamental Score` 最高只會係 60 分。
- `ROE Score`、`Gross Margin Score`、`EPS Past 5Y Score` 三個核心分數平均要至少 70；如果低過 70，`Fundamental Score` 最高只會係 75 分。
- `ROE Score` 同 `Gross Margin Score`（護城河）平均要至少 55；如果低過 55，`Fundamental Score` 最高只會係 70 分。

---

## Potential Stock 點計？

`Potential Stock` 係 boolean filter，唔係 `Fundamental Score` 嘅一部分。佢要基本面同技術面「雙強」同時成立先會標記——護城河 + 增長嘅基本面硬篩，加上動量 + 趨勢嘅技術確認。

### 基本面硬篩（全部要過）

```text
(Market Cap >= 15B)
AND
(ROE >= 18%)
AND
(EPS Past 5Y >= 15% OR Sales Past 5Y >= 18%)
AND
(Debt/Equity < 1.5)
AND
(Gross Margin >= 40%)
```

### 技術確認（全部要過）

```text
(ROC125 >= 全市場 60th percentile)
AND
(Price > EMA200)
AND
(RSI14 ∈ [40, 78])
```

兩者同時通過先標記為 `Potential Stock`。

| 類別     | 條件                         | 門檻                                   |
| -------- | ---------------------------- | -------------------------------------- |
| 市值     | `Market Cap`                 | `>= 15B`（mega/large cap + 流動性）    |
| 護城河   | `ROE`                        | `>= 18%`                               |
| 增長     | `EPS Past 5Y` 或 `Sales Past 5Y` | `>= 15%` / `>= 18%`（兩者其一）    |
| 資產負債 | `Debt/Equity`                | `< 1.5`                                |
| 定價力   | `Gross Margin`               | `>= 40%`                               |
| 動量     | `ROC125`                     | `>= 全市場 60th percentile`（用 rank） |
| 趨勢     | `EMA200Distance`             | `> 0`，等同 `Price > EMA200`           |
| 強度     | `RSI14`                      | `∈ [40, 78]`（唔超買唔超賣）           |

`ROC125` 用全市場 rank 計 percentile，避免 raw ROC 數值有 noise。如果相關資料缺失（包括某隻股票冇技術數據），`Potential Stock` 會當 `False`，唔會用 `NULL`。系統內部將百分比欄位儲存成 ratio，例如 `15%` 係 `0.15`；`Debt/Equity` 係 ratio（唔係百分比）。API 可以用 `potential_stock=true` 篩走非潛力股。

呢個 filter 係基本面 + 技術面資料做到嘅 proxy，唔等於真正 analyst revision model 或擇時模型。佢冇直接睇 30-90 日 EPS estimate revision、訂單 backlog、指引同 SEC filing；中咗 filter 只代表值得再深挖。

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
