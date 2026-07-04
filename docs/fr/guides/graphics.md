# Protocoles Graphiques

En plus de la grille de texte, kou peut décrire une image rendue à un terminal compatible afin qu'il dessine les pixels réels en ligne. Choisissez-en un avec `KOU_GRAPHICS` :

| Valeur | Protocole | Terminaux |
|-------|----------|-----------|
| `kitty` / `kitty2` | kitty APC `\e_G` graphics | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | OSC 1337 inline image | iTerm2, wezterm |
| `sixel` | DCS sixel | espace réservé — nécessite un rastériseur |
| `off` (par défaut) | aucun — génère un PNG hors bande | tous |

## Comment c'est encodé

- **Kitty.** Le PNG est encodé en base64 et transmis par fragments de ≤4096 octets dans des trames `ESC G … ST`, portant la charge de contrôle `a=T,t=d,f=100,s=<w>,v=<h>,c=<cols>,r=<rows>`. Kitty et ses clones le placent au curseur sur la zone de cellules indiquée.
- **iTerm2.** Une seule séquence `ESC ]1337;File=inline=1;width=<w>cells;height=<h>cells;size=<n>;name=<b64>:<b64-png>BEL`.

Sixel est modélisé mais pas encodé : produire un flux sixel réel nécessite un rastériseur (quantification de palette + compression sixel). Les utilisateurs souhaitant du sixel doivent passer le PNG à un encodeur dédié ; `GraphicsProtocol::supported()` le signale honnêtement.

## Comment l'utiliser

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
