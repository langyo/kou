<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>仮想端末の自動化 —— PTY + VT100 画面 + ビルド時フォント取得 + インバンドグラフィックプロトコル</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/kou/checks.yml)](https://github.com/celestia-island/kou/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-kou.docs.celestia.world-blue)](https://kou.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zhs/README.md) ·
[繁體中文](../zht/README.md) ·
**日本語** ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## はじめに

kou は、スタンドアロンの仮想端末エンジンです。PTY 管理、VT100/ANSI 画面
エミュレータ、そしてグリフを描画する画面レンダリングを備えています。
tairitsu パッケージャから抽出された vtty コアを、独立したライブラリおよび CLI
として強化したものです。

単なる PTY ラッパーとは異なる 3 つの特徴:

- **VT100 画面。** バイトストリームは [`vte`](https://crates.io/crates/vte)
  パーサーによって処理されるため、CSI カーソル移動、消去、スクロール、そして
  SGR 16 色パレットが正しく反映されます。初期プロトタイプのような「ESC を無視
  する」簡易実装ではありません。
- **ビルド時フォント取得。** kou はスクリプトごとに 1 つのフォントをビルド時に
  プレダウンロードし、共有キャッシュへ格納します。環境変数でフォントファミリーを
  上書きするかローカルファイルを固定できます。また制限のあるネットワーク環境では
  HTTP(S) プロキシ経由でダウンロードをルーティングできます。完全なリストは
  [フォントと取得](#フォントと取得)を参照してください。
- **インバンドグラフィック。** フレームを PNG にラスタライズするか、kitty
  (`kitty2`) または iTerm2 グラフィックプロトコルを通じて対応端末に記述する
  ことで、wezterm / kitty / iTerm2 / Ghostty 上で実際のピクセルをインライン
  表示できます。

## クイックスタート

### CLI

```bash
# Launch a command in a virtual terminal and drive it from a REPL.
kou launch bash --cols 80 --rows 24
# > echo hello
# > screen        # prints the current screen text
```

### ライブラリ

```rust
use kou::{FontCache, FontSet, VttyManager, render_png};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mgr = VttyManager::new();
    let id = mgr.launch("bash", None, 80, 24).await?;
    mgr.send_text(&id, "echo hello\n").await?;

    // Plain text.
    println!("{}", mgr.screenshot(&id).await?);

    // A real PNG, rendered with auto-fetched fonts (Latin + CJK fallback).
    let fonts = FontCache::load(&FontSet::from_env(), 16.0);
    let screen = mgr.screen(&id).await?;
    let png = render_png(&screen, &fonts, 16.0)?;
    std::fs::write("screen.png", png)?;
    Ok(())
}
```

## グラフィックプロトコル

| `KOU_GRAPHICS`          | プロトコル              | 対応端末               |
|-------------------------|-------------------------|------------------------|
| `kitty` / `kitty2`      | kitty APC グラフィック  | kitty, wezterm, Ghostty |
| `iterm` / `iterm2`      | OSC 1337 インライン画像 | iTerm2, wezterm        |
| `sixel`                 | DCS sixel               | (プレースホルダー — ラスタライザが必要) |
| `off` (デフォルト)      | なし — PNG を帯域外でレンダリング | すべて                 |

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};
let frame = render_graphics(&screen, &FontCache::load(&FontSet::from_env(), 16.0), 16.0,
                            GraphicsProtocol::from_env());
if let Some(escape) = frame {
    print!("{escape}"); // capable terminals render the pixels inline
}
```

## フォントと取得

kou はスクリプトごとに 1 つのフォントをビルド時に共有キャッシュへプレダウンロードします:

| スクリプト | フォント |
|------------|----------|
| Latin | Fira Code |
| CJK (中文 · 日本語 · 한국어) | Source Han Sans SC (思源黑体) |
| Arabic | Noto Naskh Arabic |
| Devanagari (हिन्दी · मराठी) | Noto Sans Devanagari |
| Thai (ไทย) | Noto Sans Thai |

`KOU_FONT_PRIMARY` / `KOU_FONT_CJK` / `KOU_FONT_ARABIC` /
`KOU_FONT_DEVANAGARI` / `KOU_FONT_THAI` でビルド時に任意のファミリーを上書き
するか、`KOU_FONT_*_PATH` でローカルファイルを固定できます。ダウンロードは
`KOU_DOWNLOAD_PROXY` (reqwest に直接渡される) を介して HTTP(S) プロキシ経由に
ルーティングできます。

| 環境変数 | 目的 |
|----------|------|
| `KOU_FONT_PRIMARY` | ラテン文字フォントファミリーを上書きします。 |
| `KOU_FONT_CJK` | CJK フォントを上書き / 無効化します (`none` で無効化)。 |
| `KOU_FONT_ARABIC` | アラビア文字フォントを上書き / 無効化します。 |
| `KOU_FONT_DEVANAGARI` | デーヴァナーガリー フォントを上書き / 無効化します。 |
| `KOU_FONT_THAI` | タイ文字フォントを上書き / 無効化します。 |
| `KOU_FONT_MIRROR` | ダウンロードホストをミラーに置き換えます。 |
| `KOU_DOWNLOAD_PROXY` | ダウンロードを HTTP(S) プロキシ経由でルーティングします (reqwest)。 |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | リクエストごとのタイムアウト (デフォルト 120)。 |
| `KOU_SKIP_FONT_FETCH` | 取得を無効にします。 |

## 開発

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## ライセンス

SySL-1.0 (Synthetic Source License)。詳細は [LICENSE](../../LICENSE) を参照
してください。
