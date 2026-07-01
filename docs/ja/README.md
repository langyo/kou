# kou

**仮想端末の自動化 —— PTY + 本物の VT100 画面 + ort 風フォント + インバンドグラフィックプロトコル。**

kou は独立した仮想端末エンジンです——PTY 管理、本物の VT100/ANSI 画面エミュレータ、
そして実際にグリフを描画する画面レンダリングを備えます。tairitsu パッケージャから
切り出して単独で強化した vtty コアです。

[`vte`](https://crates.io/crates/vte) による本物の画面（CSI/SGR 対応）、Fira Code・
Source Han・Sarasa・Smiley Sans を ort 風に自動取得するフォントパイプライン（ミラー・
プロキシ対応）、そして kitty / iTerm2 グラフィックプロトコルで画面をインライン描画する
インバンドグラフィック機能が特徴です。

完全な機能と API 表はルート [README](../../README.md) を参照してください。

> 開発中であり、API は今後変更される可能性があります。
