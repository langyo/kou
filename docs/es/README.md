# kou

**Automatización de terminal virtual — PTY + una pantalla VT100 real + tipografía estilo ort + protocolos gráficos in-band.**

kou es un motor de terminal virtual autónomo — gestión de PTY, un emulador de
pantalla VT100/ANSI real, y un renderizado de pantalla que realmente dibuja los
glifos. Es el núcleo vtty extraído del empaquetador tairitsu, reforzado como
librería y CLI propios.

Se distingue por tres cosas: una pantalla real controlada por
[`vte`](https://crates.io/crates/vte) (CSI/SGR gestionados), un pipeline de
fuentes estilo ort que obtiene Fira Code / Source Han / Sarasa / Smiley Sans bajo
demanda (con mirror/proxy), y una capacidad gráfica in-band que describe la imagen
a los terminales compatibles (kitty / iTerm2) para un renderizado de píxeles en
línea.

Para la lista completa de funciones y la API, consulta el [README](../../README.md)
raíz.

> En desarrollo; la API puede cambiar en el futuro.
