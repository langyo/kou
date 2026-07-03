# 글꼴

kou는 글꼴을 번들로 포함하지 않습니다. 대신 ONNX Runtime을 위한 [ort](https://crates.io/crates/ort)와 마찬가지로, 선별된 글꼴 패밀리를 공유 캐시로 가져와서(한 번 가져온 후 모든 실행에서 캐시됨) 투명하게 찾습니다. [`FontSet`]은 기본 고정폭 글꼴과 선택적 CJK 대체 글꼴을 쌍으로 묶습니다. 렌더러는 `ab_glyph`로 이들을 로드하고 각 코드 포인트에 대해 해당 코드 포인트를 지원하는 첫 번째 글꼴을 선택합니다 — 따라서 단일 렌더링으로 라틴 문자와 CJK를 깨짐 없이 혼합할 수 있습니다.

## 패밀리

| 슬롯 | `KOU_FONT_*` 값 | 패밀리 |
|------|--------------------|--------|
| 기본 (기본값) | `fira-code` | Fira Code |
| 기본 | `jetbrains-mono` | JetBrains Mono |
| CJK (기본값) | `sarasa` | Sarasa Mono SC (更纱黑体) |
| CJK | `sourcehansans` | Source Han Sans SC (思源黑体) |
| CJK | `smileysans` | Smiley Sans (得意黑) |
| CJK | `none` | 대체 글꼴 비활성화 |

## 해결 순서

1. **명시적 파일** — `KOU_FONT_PATH` (기본) / `KOU_FONT_CJK_PATH`.
2. **공유 캐시** — `<cache>/kou/fonts/<family>.ttf|otf`.
3. **런타임 다운로드** — `font-fetch` 기능이 공개 소스(GitHub / jsDelivr)에서 패밀리를 캐시로 가져옵니다.

## 설정

| 환경 변수 | 목적 |
|-----|---------|
| `KOU_FONT_PRIMARY` | 기본 패밀리를 재정의합니다. |
| `KOU_FONT_CJK` | CJK 패밀리를 재정의/비활성화합니다. |
| `KOU_FONT_MIRROR` | 표준 GitHub / jsDelivr 호스트를 미러로 대체합니다(예: 제한된 네트워크 환경에서 사용). |
| `KOU_DOWNLOAD_PROXY` | 글꼴 다운로드를 `http://` / `https://` / `socks5://` 프록시를 통해 라우팅합니다. |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | 요청별 타임아웃(기본값 120). |
| `KOU_SKIP_FONT_FETCH` | 런타임 가져오기를 비활성화합니다(캐시/명시적 경로만 사용). |

## 글꼴이 전혀 없는 경우

어떤 글꼴도 확인할 수 없는 경우, `FontCache`는 비어 있고 렌더러는 비어 있지 않은 각 셀에 대해 단색 블록을 그리는 것으로 저하됩니다 — 따라서 kou는 글꼴 없이도 *무언가*를 생성하며, 파일이 없어도 절대 패닉하지 않습니다.
