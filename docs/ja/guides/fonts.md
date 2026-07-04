# フォント

kouはフォントをバンドルしません。代わりに、ONNX Runtime向けの[ort](https://crates.io/crates/ort)と同様に、厳選されたフォントファミリーを共有キャッシュに取得し（初回のみ、以降はキャッシュから再利用）、透過的に配置します。[`FontSet`]は、主要な等幅フォントとオプションのCJKフォールバックをペアにします。レンダラーはこれらを`ab_glyph`で読み込み、各コードポイントに対してそれをカバーする最初のフォントを選択します——これにより、単一のレンダーで豆腐（ tofu ）なしにラテン文字とCJKを混在できます。

## ファミリー

| スロット | `KOU_FONT_*`の値 | ファミリー |
|------|--------------------|--------|
| 主要（デフォルト） | `fira-code` | Fira Code |
| 主要 | `jetbrains-mono` | JetBrains Mono |
| CJK（デフォルト） | `sarasa` | Sarasa Mono SC (更纱黑体) |
| CJK | `sourcehansans` | Source Han Sans SC (思源黑体) |
| CJK | `smileysans` | Smiley Sans (得意黑) |
| CJK | `none` | フォールバックを無効にする |

## 解決順序

1. **明示的なファイル** — `KOU_FONT_PATH`（主要）/ `KOU_FONT_CJK_PATH`。
2. **共有キャッシュ** — `<cache>/kou/fonts/<family>.ttf|otf`。
3. **ランタイムダウンロード** — `font-fetch`機能が、公開ソース（GitHub / jsDelivr）からファミリーをキャッシュに取得します。

## 設定項目

| 環境変数 | 目的 |
|-----|---------|
| `KOU_FONT_PRIMARY` | 主要ファミリーを上書きします。 |
| `KOU_FONT_CJK` | CJKファミリーを上書き/無効化します。 |
| `KOU_FONT_MIRROR` | 標準のGitHub / jsDelivrホストをミラーに置き換えます（例：制限のあるネットワーク内での使用向け）。 |
| `KOU_DOWNLOAD_PROXY` | フォントのダウンロードを`http://` / `https://` / `socks5://`プロキシ経由でルーティングします。 |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | リクエストごとのタイムアウト（デフォルト120）。 |
| `KOU_SKIP_FONT_FETCH` | ランタイム取得を無効にします（キャッシュ/明示的なパスのみ使用）。 |

## フォントが全くない場合

どのフォントも解決できない場合、`FontCache`は空になり、レンダラーは空でないセルごとに塗りつぶしブロックを描画するように縮退します——したがって、kouはフォントなしでも*何か*を生成し、ファイルが見つからない場合でも決してパニックしません。
