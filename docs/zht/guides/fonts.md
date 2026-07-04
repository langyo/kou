# 字型

kou 不捆綁字型。相反地，與 ONNX Runtime 的 [ort](https://crates.io/crates/ort) 類似，它將精選的字型系列擷取到共享快取中（僅一次，之後跨執行快取），並透明地定位。[`FontSet`] 將主要等寬字型與可選的 CJK 後備字型配對；渲染器使用 `ab_glyph` 載入它們，並對每個碼點選擇第一個涵蓋它的字型——因此單次渲染即可混合拉丁字元和 CJK，而不會出現豆腐塊。

## 系列

| 槽位 | `KOU_FONT_*` 值 | 系列 |
|------|--------------------|--------|
| 主要字型（預設） | `fira-code` | Fira Code |
| 主要字型 | `jetbrains-mono` | JetBrains Mono |
| CJK（預設） | `sarasa` | Sarasa Mono SC (更紗黑體) |
| CJK | `sourcehansans` | Source Han Sans SC (思源黑體) |
| CJK | `smileysans` | Smiley Sans (得意黑) |
| CJK | `none` | 停用後備字型 |

## 解析順序

1. **明確檔案** — `KOU_FONT_PATH`（主要字型）/ `KOU_FONT_CJK_PATH`。
2. **共享快取** — `<cache>/kou/fonts/<family>.ttf|otf`。
3. **執行時期下載** — `font-fetch` 功能從公開來源（GitHub / jsDelivr）將系列擷取到快取中。

## 設定項目

| 環境變數 | 用途 |
|-----|---------|
| `KOU_FONT_PRIMARY` | 覆寫主要字型系列。 |
| `KOU_FONT_CJK` | 覆寫/停用 CJK 字型系列。 |
| `KOU_FONT_MIRROR` | 將標準 GitHub / jsDelivr 主機替換為鏡像（例如在受限網路中使用）。 |
| `KOU_DOWNLOAD_PROXY` | 透過 `http://` / `https://` / `socks5://` 代理路由字型下載。 |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | 每次請求的逾時時間（預設 120 秒）。 |
| `KOU_SKIP_FONT_FETCH` | 停用執行時期擷取（僅使用快取/明確路徑）。 |

## 完全沒有字型

如果無法解析任何字型，`FontCache` 為空，渲染器將降級為為每個非空儲存格繪製實心區塊——因此 kou 仍能在沒有字型的情況下產生*一些內容*，並且絕不會因檔案缺失而崩潰。
