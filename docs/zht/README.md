# kou

**虛擬終端自動化——PTY + 真正的 VT100 螢幕 + ort 式字體 + 帶內圖形協議。**

kou 是一個獨立的虛擬終端引擎——PTY 管理、真正的 VT100/ANSI 螢幕模擬器，以及真正
會繪製字形的螢幕渲染。它是從 tairitsu 打包器中剝離並獨立強化的 vtty 核心。

它的特色在於：透過 [`vte`](https://crates.io/crates/vte) 解析的真正螢幕（CSI/SGR 全
支援）、ort 式自動拉取的字體管線（Fira Code、思源黑體、更紗黑體、得意黑，可選鏡像/
代理），以及能把畫面以 kitty / iTerm2 圖形協議就地渲染的帶內圖形能力。

完整特性與 API 表請見根 [README](../../README.md)。

> 仍在開發中，API 未來可能調整。
