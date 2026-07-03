# Polices

kou n'embarque pas de polices. À la place, comme [ort](https://crates.io/crates/ort) pour ONNX Runtime, elle récupère une famille sélectionnée dans un cache partagé (une fois, puis mise en cache pour toutes les exécutions) et la localise de manière transparente. Un [`FontSet`] associe une police principale à chasse fixe avec une police de secours CJK optionnelle ; le moteur de rendu les charge avec `ab_glyph` et, pour chaque point de code, choisit la première police qui le couvre — ainsi un seul rendu mélange le latin et le CJK sans tofu.

## Familles

| Emplacement | Valeur `KOU_FONT_*` | Famille |
|------|--------------------|--------|
| principale (par défaut) | `fira-code` | Fira Code |
| principale | `jetbrains-mono` | JetBrains Mono |
| CJK (par défaut) | `sarasa` | Sarasa Mono SC (更纱黑体) |
| CJK | `sourcehansans` | Source Han Sans SC (思源黑体) |
| CJK | `smileysans` | Smiley Sans (得意黑) |
| CJK | `none` | désactiver la police de secours |

## Ordre de résolution

1. **Fichier explicite** — `KOU_FONT_PATH` (principale) / `KOU_FONT_CJK_PATH`.
2. **Cache partagé** — `<cache>/kou/fonts/<family>.ttf|otf`.
3. **Téléchargement à l'exécution** — la fonctionnalité `font-fetch` récupère la famille depuis une source publique (GitHub / jsDelivr) dans le cache.

## Réglages

| Env | Objectif |
|-----|---------|
| `KOU_FONT_PRIMARY` | Remplacer la famille principale. |
| `KOU_FONT_CJK` | Remplacer / désactiver la famille CJK. |
| `KOU_FONT_MIRROR` | Remplacer l'hôte canonique GitHub / jsDelivr par un miroir (par ex. pour une utilisation derrière un réseau restrictif). |
| `KOU_DOWNLOAD_PROXY` | Router les téléchargements de polices via un proxy `http://` / `https://` / `socks5://`. |
| `KOU_DOWNLOAD_TIMEOUT_SECS` | Délai d'expiration par requête (par défaut 120). |
| `KOU_SKIP_FONT_FETCH` | Désactiver la récupération à l'exécution (utiliser uniquement le cache / les chemins explicites). |

## Aucune police du tout

Si aucune police ne peut être résolue, `FontCache` est vide et le moteur de rendu se dégrade en dessinant un bloc plein par cellule non vide — ainsi kou produit quand même *quelque chose* sans polices, et ne panique jamais en cas de fichier manquant.
