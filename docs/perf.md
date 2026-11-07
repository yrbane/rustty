# Performance : mesures et bilan

Benchmarks reproductibles (release, machine locale, lavapipe pour le rendu) :

```bash
cargo run --release -p rustty-vt --example audit
cargo run --release -p rustty-render --example audit
```

Conception et cibles : `docs/superpowers/specs/2026-11-07-optimisation-design.md`.
Les colonnes sont ajoutées chantier après chantier ; « avant » = alpha.183.

| Mesure | avant | après profil (T1) | cible |
|---|---|---|---|
| binaire release | 21,9 MiB | 14,4 MiB | ≤ 12 MiB |
| vte seul, 1 MiB texte brut (plafond) | 811 MiB/s | 511 MiB/s¹ | — |
| `Term::input` 1 MiB texte brut, 80×24, hist 10 000 | 90 MiB/s | 117 MiB/s | ≥ 200 MiB/s |
| `Term::input` SGR dense | 67 MiB/s | 134 MiB/s | ≥ 120 MiB/s |
| 1 MiB avec CR seul (coût par caractère) | 7,7 ms | 6,5 ms | — |
| `snapshot()` 200×60 | 16 µs | 5 µs | — |
| `resize` 200→120→200, hist 10 000 | 29,6 ms | 25,5 ms | ≤ 10 ms |
| redessin plein écran alternatif 200×60 | 91 µs | 84 µs | — |
| `compact_links` 5 000 liens | 1,2 ms | 1,2 ms | — |
| `pane_instances` 200×60 | 164 µs | 115 µs | — |
| `Renderer::render` 200×60 hors écran | 768 µs | 803 µs | ≤ 600 µs |

¹ Le performer vide de ce bench est inliné différemment sous LTO ; la mesure
qui compte est celle de `Term::input`.
