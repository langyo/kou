# kou

**가상 터미널 자동화 —— PTY + 진짜 VT100 화면 + ort 스타일 폰트 + 인밴드 그래픽 프로토콜.**

kou는 독립적인 가상 터미널 엔진입니다——PTY 관리, 진짜 VT100/ANSI 화면 에뮬레이터,
그리고 글리프를 실제로 그리는 화면 렌더링을 갖추고 있습니다. tairitsu 패키저에서
분리하여 독자적으로 다듬은 vtty 코어입니다.

[`vte`](https://crates.io/crates/vte) 기반의 진짜 화면(CSI/SGR 지원), Fira Code·
Source Han·Sarasa·Smiley Sans를 ort 스타일로 자동 받아오는 폰트 파이프라인(미러·프록시
지원), 그리고 kitty / iTerm2 그래픽 프로토콜로 화면을 인라인 렌더링하는 인밴드
그래픽 기능이 특징입니다.

전체 기능과 API 표는 루트 [README](../../README.md)를 참고하세요.

> 개발 중이며 API는 향후 변경될 수 있습니다.
