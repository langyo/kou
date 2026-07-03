<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>가상 터미널 자동화 — PTY + 진짜 VT100 화면 + 빌드 시간 폰트 가져오기 + 인밴드 그래픽 프로토콜</strong></p>

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

kou는 독립적인 가상 터미널 엔진입니다 — PTY 관리, 진짜 VT100/ANSI
화면 에뮬레이터, 그리고 글리프를 실제로 그리는 화면 렌더링을 갖추고 있습니다.
tairitsu 패키저에서 추출한 vtty 코어를 독립된 라이브러리와 CLI로
다듬은 것입니다.

단순한 PTY 래퍼와 차별화되는 세 가지 특징:

- **진짜 화면.** 바이트 스트림은 [`vte`](https://crates.io/crates/vte)
  파서를 통해 처리되므로, CSI 커서 이동, 지우기, 스크롤 및 SGR 16색 팔레트가
  모두 지원됩니다 — 초기 프로토타입처럼 ESC를 무시하는 수준이 아닙니다.
- **빌드 시간 폰트 가져오기.** kou는 폰트를 내장하지 않습니다. 처음 사용할 때
  큐레이션된 폰트 패밀리(라틴: Fira Code / JetBrains Mono; CJK:
  Source Han Sans / Sarasa Mono / Smiley Sans)를 공유 캐시로
  가져오며, 제한된 네트워크 환경을 위한 미러/프록시 설정을 제공합니다.
  글리프는 `ab_glyph`로 래스터화되며, 라틴을 CJK보다 먼저 처리하여
  단일 렌더링에서 여러 스크립트를 깨짐 없이 혼합합니다.
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

kou는 폰트를 내장하지 않습니다 — 빌드 시간에 큐레이션된 폰트 패밀리를 공유 캐시로 가져오며, 제한된 네트워크 환경을 위한 미러/프록시 설정을 제공합니다. 각 스크립트는 **하나**의 폰트를 선택합니다. 기본값과 대안은 다음과 같습니다:

| 스크립트 | 기본값 | 대안 |
|----------|--------|------|
| Latin | Fira Code | JetBrains Mono |
| CJK | Source Han Sans SC (思源黑体) | Sarasa Mono SC (更纱黑体), Smiley Sans (得意黑), `none` |

`KOU_FONT_PRIMARY` / `KOU_FONT_CJK`로 기본 / CJK 폰트 패밀리를 선택하거나,
`KOU_FONT_PATH` / `KOU_FONT_CJK_PATH`로 파일을 직접 지정할 수 있습니다.
우선순위: 명시적 경로 → 공유 캐시 → 런타임 다운로드
(`font-fetch` 기능, 기본 활성화).

| 환경 변수 | 용도 |
|-----|---------|
| `KOU_FONT_PRIMARY` | `fira-code` (기본값) / `jetbrains-mono` |
| `KOU_FONT_CJK` | `sourcehansans` (기본값) / `sarasa` / `smileysans` / `none` |
| `KOU_FONT_MIRROR` | GitHub / jsDelivr 호스트를 미러로 대체. |
| `KOU_DOWNLOAD_PROXY` | 폰트 다운로드를 http/https/socks 프록시를 통해 라우팅. |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | 요청별 타임아웃 (기본값 120). |
| `KOU_SKIP_FONT_FETCH` | 런타임 가져오기 비활성화. |

## 개발

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## 라이선스

SySL-1.0 (Synthetic Source License). 자세한 내용은 [LICENSE](../../LICENSE)를 참조하세요.
