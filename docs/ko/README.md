<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>가상 터미널 자동화 — PTY + VT100 화면 + 빌드 시간 폰트 가져오기 + 인밴드 그래픽 프로토콜</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/kou/checks.yml)](https://github.com/celestia-island/kou/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-kou.docs.celestia.world-blue)](https://kou.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zhs/README.md) ·
[繁體中文](../zht/README.md) ·
[日本語](../ja/README.md) ·
**한국어** ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## 소개

kou는 독립적인 가상 터미널 엔진입니다 — PTY 관리, VT100/ANSI
화면 에뮬레이터, 그리고 글리프를 그리는 화면 렌더링을 갖추고 있습니다.
tairitsu 패키저에서 추출한 vtty 코어를 독립된 라이브러리와 CLI로
다듬은 것입니다.

단순한 PTY 래퍼와 차별화되는 세 가지 특징:

- **VT100 화면.** 바이트 스트림은 [`vte`](https://crates.io/crates/vte)
  파서를 통해 처리되므로, CSI 커서 이동, 지우기, 스크롤 및 SGR 16색 팔레트가
  모두 지원됩니다 — 초기 프로토타입처럼 ESC를 무시하는 수준이 아닙니다.
- **빌드 시간 폰트 가져오기.** kou는 스크립트당 하나의 폰트를 빌드 시간에 미리
  다운로드합니다 — 라틴은 Fira Code, CJK는 Source Han Sans, 아랍어는 Noto
  Naskh Arabic, Noto Sans Devanagari, Noto Sans Thai — 공유 캐시로 가져옵니다.
  환경 변수로 패밀리를 재정의하거나 로컬 파일을 고정할 수 있습니다. 제한된
  네트워크 환경에서는 HTTP(S) 프록시(reqwest에 전달)를 통해 다운로드를
  라우팅할 수 있습니다. 글리프는 `ab_glyph`로 래스터화되며, 각 면을 순서대로
  시도하여 단일 렌더링에서 여러 스크립트를 깨짐 없이 혼합합니다.
- **인밴드 그래픽.** 프레임을 PNG로 래스터화하거나, kitty (`kitty2`)
  또는 iTerm2 그래픽 프로토콜을 통해 지원되는 터미널에 전달할 수
  있습니다 — 따라서 wezterm / kitty / iTerm2 / Ghostty에서 실제
  픽셀을 인라인으로 렌더링합니다.

## 빠른 시작

### CLI

```bash
# Launch a command in a virtual terminal and drive it from a REPL.
kou launch bash --cols 80 --rows 24
# > echo hello
# > screen        # prints the current screen text
```

### 라이브러리

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

## 그래픽 프로토콜

| `KOU_GRAPHICS` | 프로토콜 | 지원 터미널 |
|----------------|----------|-----------|
| `kitty` / `kitty2` | kitty APC 그래픽 | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 인라인 이미지 | iTerm2, wezterm |
| `sixel` | DCS sixel | (플레이스홀더 — 래스터라이저 필요) |
| `off` (기본값) | 없음 — 대역 외 PNG 렌더링 | 모두 |

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};
let frame = render_graphics(&screen, &FontCache::load(&FontSet::from_env(), 16.0), 16.0,
                            GraphicsProtocol::from_env());
if let Some(escape) = frame {
    print!("{escape}"); // capable terminals render the pixels inline
}
```

## 폰트 및 가져오기

kou는 스크립트당 하나의 폰트를 빌드 시간에 공유 캐시로 미리 다운로드합니다:

| 스크립트 | 폰트 |
|----------|------|
| Latin | Fira Code |
| CJK (中文 · 日本語 · 한국어) | Source Han Sans SC (思源黑体) |
| Arabic | Noto Naskh Arabic |
| Devanagari (हिन्दी · मराठी) | Noto Sans Devanagari |
| Thai (ไทย) | Noto Sans Thai |

`KOU_FONT_PRIMARY` / `KOU_FONT_CJK` / `KOU_FONT_ARABIC` /
`KOU_FONT_DEVANAGARI` / `KOU_FONT_THAI`로 빌드 시간에 패밀리를 재정의하거나,
`KOU_FONT_*_PATH`로 로컬 파일을 고정할 수 있습니다. 다운로드는
`KOU_DOWNLOAD_PROXY` (reqwest에 직접 전달)를 통해 HTTP(S) 프록시 경로로
라우팅할 수 있습니다.

| 환경 변수 | 용도 |
|-----|---------|
| `KOU_FONT_PRIMARY` | 라틴 폰트 패밀리를 재정의합니다. |
| `KOU_FONT_CJK` | CJK 폰트를 재정의 / 비활성화합니다 (`none`으로 비활성화). |
| `KOU_FONT_ARABIC` | 아랍어 폰트를 재정의 / 비활성화합니다. |
| `KOU_FONT_DEVANAGARI` | 데바나가리 폰트를 재정의 / 비활성화합니다. |
| `KOU_FONT_THAI` | 태국어 폰트를 재정의 / 비활성화합니다. |
| `KOU_FONT_MIRROR` | 다운로드 호스트를 미러로 대체합니다. |
| `KOU_DOWNLOAD_PROXY` | 다운로드를 HTTP(S) 프록시를 통해 라우팅합니다 (reqwest). |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | 요청별 타임아웃 (기본값 120). |
| `KOU_SKIP_FONT_FETCH` | 가져오기를 비활성화합니다. |

## 개발

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## 라이선스

SySL-1.0 (Synthetic Source License). 자세한 내용은 [LICENSE](../../LICENSE)를 참조하세요.
