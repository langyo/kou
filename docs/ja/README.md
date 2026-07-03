<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>仮想端末の自動化 —— PTY + 本物の VT100 画面 + ort 風フォント + インバンドグラフィックプロトコル。</strong></p>

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

kou は、スタンドアロンの仮想端末エンジンです。PTY 管理、本物の VT100/ANSI 画面
エミュレータ、そして実際にグリフを描画する画面レンダリングを備えています。
tairitsu パッケージャから抽出された vtty コアを、独立したライブラリおよび CLI
として強化したものです。

単なる PTY ラッパーとは異なる 3 つの特徴:

- **本物の画面。** バイトストリームは [`vte`](https://crates.io/crates/vte)
  パーサーによって処理されるため、CSI カーソル移動、消去、スクロール、そして
  SGR 16 色パレットが正しく反映されます。初期プロトタイプのような「ESC を無視
  する」簡易実装ではありません。
- **ort 風フォント。** kou はフォントを同梱しません。厳選されたフォントファミ
  リー（ラテン文字向けに Fira Code / JetBrains Mono、CJK 向けに Source Han
  Sans / Sarasa Mono / Smiley Sans）を初回使用時に共有キャッシュへ自動取得し
  ます。制限のあるネットワーク向けにミラー/プロキシ設定も利用可能です。グリフ
  は `ab_glyph` でラスタライズされ、ラテン文字が優先、CJK が後続するため、単一
  のレンダリングで tofu（文字化け）なしに複数スクリプトを混在表示できます。
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

`KOU_FONT_PRIMARY` / `KOU_FONT_CJK` でメイン / CJK フォントファミリーを選択
するか、`KOU_FONT_PATH` / `KOU_FONT_CJK_PATH` でファイルを直接指定します。
解決順序: 明示的なパス → 共有キャッシュ → 実行時ダウンロード
（`font-fetch` フィーチャー、デフォルトで有効）。

| 環境変数                     | 目的                                                      |
|------------------------------|-----------------------------------------------------------|
| `KOU_FONT_PRIMARY`           | `fira-code` (デフォルト) / `jetbrains-mono`               |
| `KOU_FONT_CJK`               | `sarasa` (デフォルト) / `sourcehansans` / `smileysans` / `none` |
| `KOU_FONT_MIRROR`            | GitHub / jsDelivr ホストをミラーに置き換えます。          |
| `KOU_DOWNLOAD_PROXY`         | フォントダウンロードを http/https/socks プロキシ経由にします。 |
| `KOU_DOWNLOAD_TIMEOUT_SECS`  | リクエストごとのタイムアウト (デフォルト 120)。           |
| `KOU_SKIP_FONT_FETCH`        | 実行時のフォント取得を無効にします。                      |

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
