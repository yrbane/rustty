//! Scénarios de bout en bout : une séquence réaliste, la grille attendue
//! figée par insta. `cargo insta review` pour accepter les changements.

use rustty_vt::Term;

fn render(cols: usize, rows: usize, input: &str) -> String {
    let mut t = Term::new(cols, rows, 50);
    t.input(input.as_bytes());
    let mut out = t.text().join("\n");
    let c = t.cursor();
    out.push_str(&format!("\n-- cursor col={} row={}", c.col, c.row));
    out
}

#[test]
fn shell_prompt_with_colors_and_clear_line() {
    insta::assert_snapshot!(render(
        20,
        3,
        "\x1b[32muser\x1b[0m@host:~$ ls\r\n\x1b[1;34mdir\x1b[0m  file\r\n\x1b[K$ "
    ));
}

#[test]
fn full_screen_app_uses_alt_screen_and_region() {
    insta::assert_snapshot!(render(
        10,
        4,
        "\x1b[?1049h\x1b[H\x1b[2J\x1b[1;1HTitle\x1b[2;4r\x1b[2;1Hl1\r\nl2\r\nl3\r\nl4\x1b[4;1Hstatus"
    ));
}

#[test]
fn box_drawing_with_dec_graphics() {
    insta::assert_snapshot!(render(6, 3, "\x1b(0lqqqqk\r\nx    x\r\nmqqqqj\x1b(B"));
}

#[test]
fn wide_chars_and_wrapping() {
    insta::assert_snapshot!(render(5, 3, "日本語テキスト"));
}

#[test]
fn garbage_input_does_not_panic() {
    let mut t = Term::new(5, 2, 10);
    let noise: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
    t.input(&noise);
    t.input(b"\x1b[999999999;999999999H\x1b[?999999h\x1b]52;c;\x1b\\");
    assert_eq!((t.grid().cols(), t.grid().rows()), (5, 2));
    let mut narrow = Term::new(1, 1, 0);
    narrow.input("漢字\u{301}".as_bytes());
    assert_eq!(narrow.grid().cols(), 1);
}
