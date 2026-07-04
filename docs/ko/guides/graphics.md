# 그래픽스 프로토콜

텍스트 그리드 위에서, kou는 렌더링된 프레임을 지원하는 터미널에 기술하여 실제 픽셀을 인라인으로 그릴 수 있습니다. `KOU_GRAPHICS`로 하나를 선택하세요:

| 값 | 프로토콜 | 터미널 |
|-------|----------|-----------|
| `kitty` / `kitty2` | kitty APC `\e_G` graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | 플레이스홀더 — 래스터라이저 필요 |
| `off` (기본값) | 없음 — 대역 외 PNG 생성 | 모두 |

## 인코딩 방식

- **Kitty.** PNG를 base64로 인코딩하여 제어 페이로드 `a=T,t=d,f=100,s=<w>,v=<h>,c=<cols>,r=<rows>`를 담은 `ESC G … ST` 프레임 내에서 ≤4096바이트 청크로 스트리밍합니다. Kitty 및 호환 터미널은 지정된 셀 영역 위의 커서 위치에 이미지를 배치합니다.
- **iTerm2.** 단일 시퀀스 `ESC ]1337;File=inline=1;width=<w>cells;height=<h>cells;size=<n>;name=<b64>:<b64-png>BEL`.

Sixel은 모델링되어 있지만 인코딩되지 않았습니다: 실제 sixel 스트림을 생성하려면 래스터라이저(팔레트 양자화 + sixel 압축)가 필요합니다. Sixel을 원하는 호출자는 PNG를 전용 인코더에 전달해야 합니다; `GraphicsProtocol::supported()`가 이를 정직하게 보고합니다.

## 사용 방법

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};

let screen = mgr.screen(&id).await?;
let fonts = FontCache::load(&FontSet::from_env(), 16.0);
if let Some(escape) = render_graphics(&screen, &fonts, 16.0, GraphicsProtocol::from_env()) {
    print!("{escape}"); // capable terminals render the pixels inline
    println!();         // advance past the placement
} else {
    // Protocol off or unsupported: fall back to a PNG.
    let png = kou::render_png(&screen, &fonts, 16.0)?;
    std::fs::write("screen.png", png)?;
}
```
