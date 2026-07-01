# Fonts

kou does not bundle fonts. Instead, like [ort](https://crates.io/crates/ort)
for ONNX Runtime, it fetches a curated family into a shared cache (once, then
cached across runs) and locates it transparently. A [`FontSet`] pairs a primary
monospace face with an optional CJK fallback; the renderer loads them with
`ab_glyph` and, for each codepoint, picks the first face that covers it — so a
single render mixes Latin and CJK without tofu.

## Families

| Slot | `KOU_FONT_*` value | Family |
|------|--------------------|--------|
| primary (default) | `fira-code` | Fira Code |
| primary | `jetbrains-mono` | JetBrains Mono |
| CJK (default) | `sarasa` | Sarasa Mono SC (更纱黑体) |
| CJK | `sourcehansans` | Source Han Sans SC (思源黑体) |
| CJK | `smileysans` | Smiley Sans (得意黑) |
| CJK | `none` | disable the fallback |

## Resolution order

1. **Explicit file** — `KOU_FONT_PATH` (primary) / `KOU_FONT_CJK_PATH`.
2. **Shared cache** — `<cache>/kou/fonts/<family>.ttf|otf`.
3. **Runtime download** — the `font-fetch` feature fetches the family from a
   public source (GitHub / jsDelivr) into the cache.

## Knobs

| Env | Purpose |
|-----|---------|
| `KOU_FONT_PRIMARY` | Override the primary family. |
| `KOU_FONT_CJK` | Override / disable the CJK family. |
| `KOU_FONT_MIRROR` | Replace the canonical GitHub / jsDelivr host with a mirror (e.g. for use behind a restrictive network). |
| `KOU_DOWNLOAD_PROXY` | Route font downloads through `http://` / `https://` / `socks5://` proxy. |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | Per-request timeout (default 120). |
| `KOU_SKIP_FONT_FETCH` | Disable runtime fetching (use cache / explicit paths only). |

## No fonts at all

If no face can be resolved, `FontCache` is empty and the renderer degrades to
drawing a solid block per non-empty cell — so kou still produces *something*
without fonts, and never panics on a missing file.
