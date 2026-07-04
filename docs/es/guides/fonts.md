# Fuentes

kou no incluye fuentes. En su lugar, al igual que [ort](https://crates.io/crates/ort) para ONNX Runtime, obtiene una familia curada en una caché compartida (una vez, luego se almacena en caché para todas las ejecuciones) y la localiza de forma transparente. Un [`FontSet`] empareja una fuente monoespaciada principal con un respaldo CJK opcional; el renderizador las carga con `ab_glyph` y, para cada punto de código, elige la primera fuente que lo cubra — de modo que un solo render mezcle latín y CJK sin caracteres tofu.

## Familias

| Ranura | Valor `KOU_FONT_*` | Familia |
|------|--------------------|--------|
| principal (predeterminada) | `fira-code` | Fira Code |
| principal | `jetbrains-mono` | JetBrains Mono |
| CJK (predeterminada) | `sarasa` | Sarasa Mono SC (更纱黑体) |
| CJK | `sourcehansans` | Source Han Sans SC (思源黑体) |
| CJK | `smileysans` | Smiley Sans (得意黑) |
| CJK | `none` | deshabilitar el respaldo |

## Orden de resolución

1. **Archivo explícito** — `KOU_FONT_PATH` (principal) / `KOU_FONT_CJK_PATH`.
2. **Caché compartida** — `<cache>/kou/fonts/<family>.ttf|otf`.
3. **Descarga en tiempo de ejecución** — la característica `font-fetch` obtiene la familia desde una fuente pública (GitHub / jsDelivr) a la caché.

## Controles

| Env | Propósito |
|-----|---------|
| `KOU_FONT_PRIMARY` | Anular la familia principal. |
| `KOU_FONT_CJK` | Anular / deshabilitar la familia CJK. |
| `KOU_FONT_MIRROR` | Reemplazar el host canónico de GitHub / jsDelivr con un espejo (por ejemplo, para usar detrás de una red restrictiva). |
| `KOU_DOWNLOAD_PROXY` | Enrutar las descargas de fuentes a través de un proxy `http://` / `https://` / `socks5://`. |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | Tiempo de espera por solicitud (predeterminado 120). |
| `KOU_SKIP_FONT_FETCH` | Deshabilitar la obtención en tiempo de ejecución (usar solo caché / rutas explícitas). |

## Sin fuentes en absoluto

Si no se puede resolver ninguna fuente, `FontCache` está vacío y el renderizador se degrada dibujando un bloque sólido por cada celda no vacía — así que kou aún produce *algo* sin fuentes, y nunca entra en pánico por un archivo faltante.
