<p align="center"><img src="../logo.webp" alt="kou" width="240" /></p>

<h1 align="center">kou</h1>

<p align="center"><strong>Automatisation de terminal virtuel — PTY + un vrai écran VT100 + récupération des polices à la compilation + protocoles graphiques in-band</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/kou/checks.yml)](https://github.com/celestia-island/kou/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-kou.docs.celestia.world-blue)](https://kou.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zhs/README.md) ·
[繁體中文](../zht/README.md) ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
**Français** ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## Introduction

kou est un moteur de terminal virtuel autonome — gestion PTY, un émulateur
d'écran VT100/ANSI réel, et un rendu d'écran qui dessine réellement les glyphes.
Il s'agit du cœur vtty extrait de l'empaqueteur tairitsu, durci en une
bibliothèque et un CLI à part entière.

Trois choses le distinguent d'un simple wrapper PTY :

- **Un vrai écran.** Le flux d'octets est traité par l'analyseur
  [`vte`](https://crates.io/crates/vte), donc les déplacements de curseur CSI,
  l'effacement, le défilement et la palette SGR 16 couleurs sont respectés —
  pas le bouchon « jeter ESC au sol » du premier prototype.
- **Récupération des polices à la compilation.** kou n'embarque pas de polices ; il récupère une famille
  sélectionnée (Fira Code / JetBrains Mono pour le latin ; Source Han Sans /
  Sarasa Mono / Smiley Sans pour le CJK) dans un cache partagé à la première
  utilisation, avec des options miroir/proxy pour les réseaux restrictifs. Les
  glyphes sont rastérisés avec `ab_glyph`, le latin avant le CJK, de sorte qu'un
  seul rendu mélange les écritures sans tofu.
- **Graphiques in-band.** Une trame peut être rastérisée en PNG, ou décrite à un
  terminal compatible via le protocole graphique kitty (`kitty2`) ou iTerm2 —
  ainsi wezterm / kitty / iTerm2 / Ghostty affichent les vrais pixels en ligne.

## Démarrage rapide

### CLI

```bash
# Launch a command in a virtual terminal and drive it from a REPL.
kou launch bash --cols 80 --rows 24
# > echo hello
# > screen        # prints the current screen text
```

### Bibliothèque

```rust
use kou::{FontCache, FontSet, VttyManager, render_png};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mgr = VttyManager::new();
    let id = mgr.launch("bash", None, 80, 24).await?;
    mgr.send_text(&id, "echo hello\n").await?;

    // Plain text.
    println!("{}", mgr.screenshot(&id).await?);

    // A real PNG, rendered with auto-fetched fonts (Latin + CJK fallback).
    let fonts = FontCache::load(&FontSet::from_env(), 16.0);
    let screen = mgr.screen(&id).await?;
    let png = render_png(&screen, &fonts, 16.0)?;
    std::fs::write("screen.png", png)?;
    Ok(())
}
```

## Protocoles graphiques

| `KOU_GRAPHICS` | Protocole | Terminaux |
|----------------|-----------|-----------|
| `kitty` / `kitty2` | graphiques APC kitty | kitty, wezterm, Ghostty |
| `iterm` / `iterm2` | image inline OSC 1337 | iTerm2, wezterm |
| `sixel` | DCS sixel | (réservé — nécessite un rastériseur) |
| `off` (par défaut) | aucun — rend un PNG hors bande | tous |

```rust
use kou::{FontCache, FontSet, GraphicsProtocol, VttyManager, render_graphics};
let frame = render_graphics(&screen, &FontCache::load(&FontSet::from_env(), 16.0), 16.0,
                            GraphicsProtocol::from_env());
if let Some(escape) = frame {
    print!("{escape}"); // capable terminals render the pixels inline
}
```

## Polices et récupération

kou ne regroupe pas de polices — il récupère une famille sélectionnée dans un
cache partagé à la compilation, avec des options miroir/proxy pour les réseaux
restrictifs. Chaque écriture sélectionne **une** police ; les valeurs par défaut
et alternatives sont :

| Écriture | Défaut | Alternatives |
|----------|--------|--------------|
| Latin | Fira Code | JetBrains Mono |
| CJK | Source Han Sans SC (思源黑体) | Sarasa Mono SC (更纱黑体), Smiley Sans (得意黑), `none` |

Choisissez la famille principale / CJK avec `KOU_FONT_PRIMARY` /
`KOU_FONT_CJK`, ou épinglez des fichiers avec `KOU_FONT_PATH` /
`KOU_FONT_CJK_PATH`. Ordre de résolution : chemin explicite → cache partagé →
téléchargement à l'exécution (la fonctionnalité `font-fetch`, activée par
défaut).

| Env | Rôle |
|-----|------|
| `KOU_FONT_PRIMARY` | `fira-code` (par défaut) / `jetbrains-mono` |
| `KOU_FONT_CJK` | `sourcehansans` (par défaut) / `sarasa` / `smileysans` / `none` |
| `KOU_FONT_MIRROR` | Remplace l'hôte GitHub / jsDelivr par un miroir. |
| `KOU_DOWNLOAD_PROXY` | Achemine les téléchargements de polices via un proxy http/https/socks. |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | Délai d'expiration par requête (par défaut 120). |
| `KOU_SKIP_FONT_FETCH` | Désactive la récupération à l'exécution. |

## Développement

```bash
cargo check --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Licence

SySL-1.0 (Synthetic Source License). Voir [LICENSE](../../LICENSE).
