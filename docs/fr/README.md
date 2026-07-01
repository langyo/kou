# kou

**Automatisation de terminal virtuel — PTY + un vrai écran VT100 + polices façon ort + protocoles graphiques in-band.**

kou est un moteur de terminal virtuel autonome — gestion PTY, un émulateur
d'écran VT100/ANSI réel, et un rendu d'écran qui dessine réellement les glyphes.
C'est le cœur vtty extrait du empaqueteur tairitsu, durci en une bibliothèque et
un CLI à part entière.

Trois choses le distinguent : un vrai écran piloté par [`vte`](https://crates.io/crates/vte)
(CSI/SGR gérés), un pipeline de polices ort-style qui récupère Fira Code / Source
Han / Sarasa / Smiley Sans à la demande (avec miroir/proxy), et une capacité
graphique in-band qui décrit l'image aux terminaux capables (kitty / iTerm2) pour
un rendu pixel inline.

Pour la liste complète des fonctionnalités et de l'API, voir le
[README](../../README.md) racine.

> En cours de développement ; l'API peut changer à l'avenir.
