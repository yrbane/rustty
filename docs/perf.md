# Performance : mesures et bilan

Benchmarks reproductibles (release, machine locale, lavapipe pour le rendu) :

```bash
cargo run --release -p rustty-vt --example audit
cargo run --release -p rustty-render --example audit
```

Conception et cibles : `docs/superpowers/specs/2026-11-07-optimisation-design.md`.
Les colonnes sont ajoutées chantier après chantier ; « avant » = alpha.183.

| Mesure | avant | après profil (T1) | après impression (T2) | après reflow (T3) | après GPU (T4) | final (T7) | cible |
|---|---|---|---|---|---|---|---|
| binaire release | 21,9 MiB | 14,4 MiB | | 14,4 MiB | 14,4 MiB | 14,4 MiB | **14,4 MiB** | ≤ 12 MiB |
| vte seul, 1 MiB texte brut (plafond) | 811 MiB/s | 511 MiB/s¹ | | 590 MiB/s | 590 MiB/s | 590 MiB/s | 506 MiB/s | — |
| `Term::input` 1 MiB texte brut, 80×24, hist 10 000 | 90 MiB/s | 117 MiB/s | | 175 MiB/s | 165 MiB/s | 165 MiB/s | **163 MiB/s** | ≥ 200 MiB/s |
| `Term::input` SGR dense | 67 MiB/s | 134 MiB/s | | 165 MiB/s | 155 MiB/s | 155 MiB/s | **152 MiB/s** | ≥ 120 MiB/s |
| 1 MiB avec CR seul (coût par caractère) | 7,7 ms | 6,5 ms | | 4,2 ms | 4,4 ms | 4,4 ms | **4,6 ms** | — |
| `snapshot()` 200×60 | 16 µs | 5 µs | | 5 µs | 5 µs | 5 µs | **5 µs** | — |
| `resize` 200→120→200, hist 10 000 | 29,6 ms | 25,5 ms | | 20,2 ms | **7,0 ms** | 7,0 ms | **6,7 ms** | ≤ 10 ms |
| redessin plein écran alternatif 200×60 | 91 µs | 84 µs | | 58 µs | 57 µs | 57 µs | **61 µs** | — |
| `compact_links` 5 000 liens | 1,2 ms | 1,2 ms | | 1,2 ms | 1,2 ms | 1,2 ms | **1,1 ms** | — |
| `pane_instances` 200×60 | 164 µs | 115 µs | | 81 µs | 81 µs | 89 µs | **86 µs** | — |
| `Renderer::render` 200×60 hors écran | 768 µs | 803 µs | | 489 µs | 489 µs | **477 µs** | **429 µs** | ≤ 600 µs |

¹ Le performer vide de ce bench est inliné différemment sous LTO ; la mesure
qui compte est celle de `Term::input`.

Depuis T2, chaque mesure est le meilleur de 5 répétitions (le bruit de la
machine ne fait que rallonger) : les colonnes « avant » et « T1 » sont donc
légèrement pessimistes par rapport aux suivantes.

T2 : chemin rapide de `put_char` (caractère étroit, cellule ordinaire : une
lecture, une écriture), `Line::set` sans recherche dans la table des
combinants quand elle est vide, `Grid::scroll_up` sans clone, jeu ASCII et
largeur ASCII court-circuités, découpeur APC qui saute jusqu'au prochain ESC.
Le texte brut plafonne à ~175 MiB/s : vte seul coûte 1,7 ns/octet, le reste
(2,4 ns) est la lecture et l'écriture de 16 octets par cellule.

T3 : le reflow garde des entrées de 16 octets (les combinants sont à part,
indexés par entrée) et copie par tranche les lignes logiques sans caractère
large ni curseur, de loin les plus nombreuses ; dans la fenêtre, un glisser
de barre ne redimensionne plus les terminaux qu'une fois par image (le
relayout est différé au rendu). Resize 20 → 7 ms. Les colonnes de T3 pour
l'impression sont dans le bruit de la machine (±10 %).

T4 : les cinq tampons d'instances sont persistants (`write_buffer`, doublés
quand une image ne tient plus, jamais réduits), l'uniforme du viewport et
son bind group sont partagés par les trois pipelines et réécrits sur place,
le bind group de l'atlas n'est refait qu'avec l'atlas. Objets GPU créés par
image en régime établi : 10 → 0 (`Renderer::gpu_objects`, testé).

T5 (structure, pas de cible de vitesse) : `pane_instances` 81 → 109 µs et
`Renderer::render` 477 → 486 µs après découpage, dans le bruit de la
machine ; `pipeline/common.rs` porte le shader, le pipeline instancié, le
layout de texture et l'échantillonneur : zéro fenêtre de quatre lignes
dupliquée entre les trois pipelines (10 avant).

T7 (hygiène) : `base64` 0.22 → 0.23 (plus de doublon dans l'arbre des
dépendances), un seul `snapshot()` par mouvement de souris partagé par le
correcteur et les liens (deux avant), `clippy::perf` à zéro (le groupe est
actif par défaut).

## Bilan

| | avant | après |
|---|---|---|
| binaire release | 21,9 MiB | 14,4 MiB (−34 %) |
| débit texte brut | 90 MiB/s | ~165 MiB/s (×1,8) |
| débit SGR dense | 67 MiB/s | ~155 MiB/s (×2,3) |
| resize avec 10 000 lignes d'historique | 29,6 ms | 6,7 ms (×4,4) ; et un seul par image pendant un glisser |
| rendu hors écran 200×60 | 768 µs | 429 µs (×1,8) |
| objets GPU créés par image (régime établi) | 10 | 0 |
| fonctions ≥ 100 lignes | 4 | 0 (gardé par `tests/code_shape.rs`) |
| fenêtres dupliquées entre pipelines | 10 | 0 |

Cibles manquées : binaire ≤ 12 MiB (14,4 : le reste est wgpu, naga et la
police embarquée) et texte brut ≥ 200 MiB/s (~165 : vte seul coûte déjà
1,7 ns/octet, le reste est l'écriture de 16 octets par cellule).
