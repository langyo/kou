# 字体

kou 不捆绑字体。相反，与 ONNX Runtime 的 [ort](https://crates.io/crates/ort) 类似，它将精选的字体系列获取到共享缓存中（仅一次，之后跨运行缓存），并透明地定位。[`FontSet`] 将主等宽字体与可选的 CJK 回退字体配对；渲染器使用 `ab_glyph` 加载它们，并对每个码点选择第一个覆盖它的字体——因此单次渲染即可混合拉丁字符和 CJK，而不会出现豆腐块。

## 系列

| 槽位 | `KOU_FONT_*` 值 | 系列 |
|------|--------------------|--------|
| 主字体（默认） | `fira-code` | Fira Code |
| 主字体 | `jetbrains-mono` | JetBrains Mono |
| CJK（默认） | `sarasa` | Sarasa Mono SC (更纱黑体) |
| CJK | `sourcehansans` | Source Han Sans SC (思源黑体) |
| CJK | `smileysans` | Smiley Sans (得意黑) |
| CJK | `none` | 禁用回退字体 |

## 解析顺序

1. **显式文件** — `KOU_FONT_PATH`（主字体）/ `KOU_FONT_CJK_PATH`。
2. **共享缓存** — `<cache>/kou/fonts/<family>.ttf|otf`。
3. **运行时下载** — `font-fetch` 功能从公共源（GitHub / jsDelivr）将系列获取到缓存中。

## 配置项

| 环境变量 | 用途 |
|-----|---------|
| `KOU_FONT_PRIMARY` | 覆盖主字体系列。 |
| `KOU_FONT_CJK` | 覆盖/禁用 CJK 字体系列。 |
| `KOU_FONT_MIRROR` | 将标准 GitHub / jsDelivr 主机替换为镜像（例如在受限网络中使用）。 |
| `KOU_DOWNLOAD_PROXY` | 通过 `http://` / `https://` / `socks5://` 代理路由字体下载。 |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | 每次请求的超时时间（默认 120 秒）。 |
| `KOU_SKIP_FONT_FETCH` | 禁用运行时获取（仅使用缓存/显式路径）。 |

## 完全没有字体

如果无法解析任何字体，`FontCache` 为空，渲染器将降级为为每个非空单元格绘制实心块——因此 kou 仍能在没有字体的情况下生成*一些内容*，并且绝不会因文件缺失而崩溃。
