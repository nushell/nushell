// Regression tests for https://github.com/nushell/nushell/issues/17802
// `ls` panics on files with zero-width space characters in their names.
//
// The root cause is that tabled's Wrap::wrap / Truncate::truncate performs
// byte-level string slicing that can land inside multi-byte UTF-8 characters
// (e.g., \u{200b} is 3 bytes), causing a panic.

mod common;

use common::{TestCase, create_table};
use nu_protocol::TrimStrategy;
use nu_table::{TableTheme as theme, clean_charset, string_truncate, string_wrap};
use tabled::grid::records::vec_records::Text;

// ────────────────────────────────────────────────────────────────────
// 1. clean_charset strips zero-width characters
// ────────────────────────────────────────────────────────────────────

#[test]
fn clean_charset_strips_zero_width_space() {
    let input = "\u{200b}hello";
    let cleaned = clean_charset(input);
    assert_eq!(cleaned, "hello");
}

#[test]
fn clean_charset_strips_zero_width_joiner() {
    let input = "he\u{200d}llo";
    let cleaned = clean_charset(input);
    assert_eq!(cleaned, "hello");
}

#[test]
fn clean_charset_strips_zero_width_non_joiner() {
    let input = "he\u{200c}llo";
    let cleaned = clean_charset(input);
    assert_eq!(cleaned, "hello");
}

#[test]
fn clean_charset_strips_bom() {
    let input = "\u{feff}hello";
    let cleaned = clean_charset(input);
    assert_eq!(cleaned, "hello");
}

#[test]
fn clean_charset_strips_soft_hyphen() {
    let input = "hel\u{00ad}lo";
    let cleaned = clean_charset(input);
    assert_eq!(cleaned, "hello");
}

#[test]
fn clean_charset_strips_word_joiner() {
    let input = "hello\u{2060}world";
    let cleaned = clean_charset(input);
    assert_eq!(cleaned, "helloworld");
}

#[test]
fn clean_charset_strips_multiple_zero_width_chars() {
    let input = "\u{200b}\u{200c}\u{200d}hello\u{feff}world\u{2060}";
    let cleaned = clean_charset(input);
    assert_eq!(cleaned, "helloworld");
}

#[test]
fn clean_charset_preserves_normal_text() {
    let input = "normal text 123!@#";
    let cleaned = clean_charset(input);
    assert_eq!(cleaned, "normal text 123!@#");
}

// ────────────────────────────────────────────────────────────────────
// 2. string_wrap doesn't panic on zero-width characters
// ────────────────────────────────────────────────────────────────────

#[test]
fn string_wrap_zero_width_space_no_panic() {
    // This is the exact scenario from issue #17802:
    // filename starts with \u{200b} (3 bytes, 0 display width)
    let text = "\u{200b} [7577464208380415287].mp3";
    // Should not panic, regardless of width
    let _ = string_wrap(text, 5, false);
    let _ = string_wrap(text, 10, false);
    let _ = string_wrap(text, 1, false);
}

#[test]
fn string_wrap_mixed_zero_width_and_normal() {
    let text = "a\u{200b}b\u{200c}c\u{200d}d";
    let _ = string_wrap(text, 2, false);
    let _ = string_wrap(text, 1, true);
}

// ────────────────────────────────────────────────────────────────────
// 3. string_truncate doesn't panic on zero-width characters
// ────────────────────────────────────────────────────────────────────

#[test]
fn string_truncate_zero_width_space_no_panic() {
    let text = "\u{200b} [7577464208380415287].mp3";
    let _ = string_truncate(text, 5);
    let _ = string_truncate(text, 1);
}

// ────────────────────────────────────────────────────────────────────
// 4. Full table rendering with zero-width characters doesn't panic
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_with_zero_width_space_in_cell_no_panic() {
    // Simulate what ls would produce: a filename with zero-width space
    let row = vec![
        Text::new("\u{200b} [7577464208380415287].mp3".to_string()),
        Text::new("1.2 MB".to_string()),
        Text::new("audio/mpeg".to_string()),
    ];
    let data = vec![row];

    // Try various narrow widths that would trigger wrapping/truncation
    for width in [10, 20, 30, 50, 80] {
        let table = create_table(
            data.clone(),
            TestCase::new(width).theme(theme::rounded()),
        );
        // Must not panic; result can be None if too narrow
        let _ = table;
    }
}

#[test]
fn table_with_zero_width_space_truncate_strategy() {
    let row = vec![
        Text::new("\u{200b}filename.txt".to_string()),
        Text::new("100 B".to_string()),
    ];
    let data = vec![row];

    let table = create_table(
        data,
        TestCase::new(20)
            .theme(theme::rounded())
            .trim(TrimStrategy::truncate(Some("...".to_string()))),
    );
    let _ = table;
}

#[test]
fn table_with_zero_width_space_wrap_strategy() {
    let row = vec![
        Text::new("\u{200b}filename.txt".to_string()),
        Text::new("100 B".to_string()),
    ];
    let data = vec![row];

    let table = create_table(
        data,
        TestCase::new(20)
            .theme(theme::rounded())
            .trim(TrimStrategy::wrap(true)),
    );
    let _ = table;
}
