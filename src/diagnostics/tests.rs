use super::*;

#[test]
fn display_escapes_terminal_controls() {
    // Regression: an ESC sequence in a layer name reached the terminal.
    let d = Diagnostic::warning("Card\u{1b}]52;c;x\u{7}", "bad\u{1b}[2K\u{202e}");
    let shown = d.to_string();
    assert!(
        !shown.contains('\u{1b}') && !shown.contains('\u{7}'),
        "{shown:?}"
    );
    assert!(!shown.contains('\u{202e}'), "{shown:?}");
    assert!(shown.contains("\\u{1b}]52"), "{shown}");
}
