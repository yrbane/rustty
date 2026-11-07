//! Micro-benchmarks d'audit (lancer en release).
use std::time::Instant;

use rustty_vt::Term;

/// Meilleur temps par itération sur `REPS` répétitions : le bruit de la
/// machine (fréquence, autres processus) ne fait que rallonger, jamais
/// raccourcir.
const REPS: u32 = 5;
fn timed<R>(label: &str, iters: u32, mut f: impl FnMut() -> R) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..REPS {
        let t0 = Instant::now();
        for _ in 0..iters {
            std::hint::black_box(f());
        }
        best = best.min(t0.elapsed().as_secs_f64() / f64::from(iters));
    }
    println!("{label:<48} {:>10.1} µs", best * 1e6);
    best
}

fn main() {
    vte_baseline();
    split_costs();
    println!(
        "size_of Cell={} Line={} Snapshot={}",
        std::mem::size_of::<rustty_vt::Cell>(),
        std::mem::size_of::<rustty_vt::Line>(),
        std::mem::size_of::<rustty_vt::Snapshot>()
    );
    // 1. débit de parsing : texte brut 1 MiB avec retours ligne (80 col)
    let plain: Vec<u8> = (0..13_000)
        .flat_map(|i| format!("ligne {i:06} {}\r\n", "abcdefghij".repeat(6)).into_bytes())
        .collect();
    let mut t = Term::new(80, 24, 10_000);
    let s = timed("parse 1 MiB texte brut (80x24, hist 10k)", 5, || {
        t.input(&plain)
    });
    println!("  → {:.0} MiB/s", plain.len() as f64 / 1048576.0 / s);
    // 2. SGR dense (couleurs 24 bits chaque mot)
    let sgr: Vec<u8> = (0..8_000)
        .flat_map(|i| {
            format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;1;2;3mmot{i:05} \x1b[0m\x1b[1;4mgras\x1b[0m \r\n",
                i % 255,
                (i * 7) % 255,
                (i * 13) % 255
            )
            .into_bytes()
        })
        .collect();
    let s = timed("parse SGR dense (8k lignes)", 5, || t.input(&sgr));
    println!("  → {:.0} MiB/s", sgr.len() as f64 / 1048576.0 / s);
    // 3. snapshot grande grille avec historique
    let mut big = Term::new(200, 60, 10_000);
    let fill: Vec<u8> = (0..12_000)
        .flat_map(|i| format!("\x1b[3{}m{i:06} {}\r\n", i % 8, "x".repeat(150)).into_bytes())
        .collect();
    big.input(&fill);
    timed("snapshot 200x60 (hist 10k)", 2000, || big.snapshot());
    big.scroll_display(30);
    timed("snapshot 200x60 défilé de 30 (hist)", 2000, || {
        big.snapshot()
    });
    big.scroll_display_to_bottom();
    // 4. redimensionnement avec reflow
    timed("resize 200→120→200 cols (hist 10k)", 5, || {
        big.resize(120, 60);
        big.resize(200, 60);
    });
    // 5. écran alternatif plein redessiné (vim) : 60 lignes × 200 col avec curseur
    let mut alt = Term::new(200, 60, 10_000);
    alt.input(b"\x1b[?1049h");
    let frame: Vec<u8> = (1..=60)
        .flat_map(|r| format!("\x1b[{r};1H\x1b[K{}", "ligne pleine ".repeat(15)).into_bytes())
        .collect();
    timed("redessin plein écran alt 200x60 (vim)", 200, || {
        alt.input(&frame)
    });
    // 6. compactage des liens
    let mut links = Term::new(100, 40, 10_000);
    for i in 0..5000 {
        links.input(format!("\x1b]8;;https://h/{i}\x1b\\lien{i}\x1b]8;;\x1b\\\r\n").as_bytes());
    }
    timed("compact_links (5k liens, hist 5k)", 50, || {
        links.compact_links()
    });
    timed("text() 200x60", 200, || big.text());
}

#[allow(dead_code)]
mod baseline {
    pub struct Noop(pub u64);
    impl vte::Perform for Noop {
        fn print(&mut self, c: char) {
            self.0 += c as u64;
        }
    }
}
#[allow(dead_code)]
pub fn vte_baseline() {
    let plain: Vec<u8> = (0..13_000)
        .flat_map(|i| format!("ligne {i:06} {}\r\n", "abcdefghij".repeat(6)).into_bytes())
        .collect();
    let mut p = vte::Parser::new();
    let mut n = baseline::Noop(0);
    let t0 = Instant::now();
    for _ in 0..5 {
        p.advance(&mut n, &plain);
    }
    let s = t0.elapsed().as_secs_f64() / 5.0;
    println!(
        "vte seul, 1 MiB texte brut : {:.0} MiB/s ({})",
        plain.len() as f64 / 1048576.0 / s,
        n.0 % 7
    );
}

#[allow(dead_code)]
pub fn split_costs() {
    // Même volume : (a) une ligne de 1 MiB sans retour (wrap automatique, 80 col),
    // (b) 13k lignes de 80 avec CRLF, (c) 80x24 sans historique, (d) ASCII sans wrap (curseur ramené)
    let a: Vec<u8> = vec![b'a'; 1_040_000];
    let b: Vec<u8> = (0..13_000)
        .flat_map(|_| {
            let mut v = vec![b'a'; 78];
            v.extend_from_slice(b"\r\n");
            v
        })
        .collect();
    let mut t = Term::new(80, 24, 10_000);
    timed("1 MiB 'a' sans CRLF (wrap) hist 10k", 5, || t.input(&a));
    timed("1 MiB 'a' avec CRLF hist 10k", 5, || t.input(&b));
    let mut t0 = Term::new(80, 24, 0);
    timed("1 MiB 'a' avec CRLF hist 0", 5, || t0.input(&b));
    let c: Vec<u8> = (0..13_000)
        .flat_map(|_| {
            let mut v = vec![b'a'; 78];
            v.extend_from_slice(b"\r");
            v
        })
        .collect();
    timed("1 MiB 'a' avec CR seul (pas de scroll)", 5, || t0.input(&c));
}
