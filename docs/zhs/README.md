# kou

**虚拟终端自动化——PTY + 真正的 VT100 屏幕 + ort 式字体 + 带内图形协议。**

kou 是一个独立的虚拟终端引擎——PTY 管理、真正的 VT100/ANSI 屏幕模拟器，以及真正
会绘制字形的屏幕渲染。它是从 tairitsu 打包器中剥离并独立强化的 vtty 核心。

它区别于普通 PTY 封装的三点：

- **真正的屏幕。** 字节流经由 [`vte`](https://crates.io/crates/vte) 解析，因此 CSI 光标
  移动、擦除、滚动以及 SGR 16 色调色板都被正确处理——而非早期原型那种“把 ESC 直接
  丢掉”的做法。
- **ort 式字体。** kou 不内置字体；它会在首次使用时把精选字体族（Latin 用 Fira Code /
  JetBrains Mono；CJK 用思源黑体 / 更纱黑体 / 得意黑）拉取到共享缓存，并提供镜像/
  代理开关以适配受限网络。字形由 `ab_glyph` 光栅化，Latin 优先、CJK 兜底，让单次渲染
  就能混排多种文字而不出现豆腐块。
- **带内图形。** 一帧既可光栅化为 PNG，也可经由 kitty（`kitty2`）或 iTerm2 图形协议
  描述给受支持的终端——这样 wezterm / kitty / iTerm2 / Ghostty 能就地渲染真实像素。

完整特性与 API 表请见根 [README](../../README.md)。

> 仍在开发中，API 未来可能调整。
