use nu_color_config::StyleComputer;

use tabled::{
    grid::{
        ansi::{ANSIBuf, ANSIStr},
        records::vec_records::Text,
        util::string::get_text_width,
    },
    settings::{
        Color,
        width::{Truncate, Wrap},
    },
};

use crate::common::get_leading_trailing_space_style;

pub fn string_width(text: &str) -> usize {
    get_text_width(text)
}

pub fn string_wrap(text: &str, width: usize, keep_words: bool) -> String {
    if text.is_empty() {
        return String::new();
    }

    let text_width = string_width(text);
    if text_width <= width {
        return text.to_owned();
    }

    Wrap::wrap(text, width, keep_words)
}

pub fn string_expand(text: &str, width: usize) -> String {
    use std::{borrow::Cow, iter::repeat_n};
    use tabled::grid::util::string::{get_line_width, get_lines};

    get_lines(text)
        .map(|line| {
            let length = get_line_width(&line);

            if length < width {
                let mut line = line.into_owned();
                let remain = width - length;
                line.extend(repeat_n(' ', remain));
                Cow::Owned(line)
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn string_truncate(text: &str, width: usize) -> String {
    let line = match text.lines().next() {
        Some(line) => line,
        None => return String::new(),
    };

    Truncate::truncate(line, width).into_owned()
}

pub fn clean_charset(text: &str) -> String {
    // allocating at least the text size,
    // in most cases the buf will be a copy of text anyhow.
    let mut buf = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '\r' => continue,
            '\t' => {
                buf.push(' ');
                buf.push(' ');
                buf.push(' ');
                buf.push(' ');
            }
            c => {
                buf.push(c);
            }
        }
    }

    // Security: Strip dangerous ANSI escape sequences from user data to prevent
    // terminal injection attacks (see https://github.com/nushell/nushell/issues/12725).
    //
    // We only strip non-color sequences (cursor movement, screen clearing, etc.)
    // while preserving SGR color/style sequences (ESC[...m) since those are used
    // intentionally by Nushell for LS_COLORS and other internal styling.
    // Fast path: skip if no ESC byte (0x1B) is present.
    if buf.as_bytes().contains(&0x1B) {
        strip_dangerous_ansi_sequences(&buf)
    } else {
        buf
    }
}

/// Strip dangerous (non-color) ANSI escape sequences while preserving SGR
/// color/style sequences (CSI ... m).
///
/// This removes cursor movement, screen clearing, scrolling, and other
/// potentially dangerous terminal control sequences that could be used for
/// terminal injection attacks. Color sequences (those ending with 'm') are
/// preserved because Nushell uses them internally for LS_COLORS styling.
fn strip_dangerous_ansi_sequences(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c == '\x1b' {
            // Check for CSI sequence: ESC[
            if chars.peek().is_some_and(|&(_, next)| next == '[') {
                chars.next(); // consume '['
                // Collect parameter bytes and find the final byte
                if chars.peek().is_none() {
                    // ESC[ at end of string — skip it
                    continue;
                }
                let mut final_byte = None;
                while let Some(&(_, ch)) = chars.peek() {
                    if ch.is_ascii_alphabetic() || ch == '@' || ch == '`' {
                        final_byte = Some(ch);
                        chars.next(); // consume final byte
                        break;
                    }
                    chars.next(); // consume parameter/intermediate bytes
                }
                // Only preserve SGR sequences (final byte 'm' = color/style)
                if final_byte == Some('m') {
                    result.push_str(&text[i..chars.peek().map_or(text.len(), |&(p, _)| p)]);
                }
                // All other CSI sequences (cursor movement, clear screen, etc.) are dropped
            } else if chars.peek().is_some_and(|&(_, next)| next == ']') {
                // OSC sequence: ESC] ... ST — skip entirely
                chars.next(); // consume ']'
                while let Some((_, ch)) = chars.next() {
                    if ch == '\x07' {
                        break; // BEL terminates OSC
                    }
                    if ch == '\x1b' && chars.peek().is_some_and(|&(_, next)| next == '\\') {
                        chars.next(); // consume '\\'
                        break; // ST terminates OSC
                    }
                }
            } else {
                // Other ESC sequences (e.g., ESC followed by single char) — skip
                chars.next(); // consume the byte after ESC
            }
        } else {
            result.push(c);
        }
    }

    result
}

pub fn colorize_space(data: &mut [Vec<Text<String>>], style_computer: &StyleComputer<'_>) {
    let style = match get_leading_trailing_space_style(style_computer).color_style {
        Some(color) => color,
        None => return,
    };

    let style = ANSIBuf::from(convert_style(style));
    let style = style.as_ref();
    if style.is_empty() {
        return;
    }

    colorize_list(data, style, style);
}

pub fn colorize_space_str(text: &mut String, style_computer: &StyleComputer<'_>) {
    let style = match get_leading_trailing_space_style(style_computer).color_style {
        Some(color) => color,
        None => return,
    };

    let style = ANSIBuf::from(convert_style(style));
    let style = style.as_ref();
    if style.is_empty() {
        return;
    }

    *text = colorize_space_one(text, style, style);
}

fn colorize_list(data: &mut [Vec<Text<String>>], lead: ANSIStr<'_>, trail: ANSIStr<'_>) {
    for row in data.iter_mut() {
        for cell in row {
            let buf = colorize_space_one(cell.as_ref(), lead, trail);
            *cell = Text::new(buf);
        }
    }
}

fn colorize_space_one(text: &str, lead: ANSIStr<'_>, trail: ANSIStr<'_>) -> String {
    use fancy_regex::Captures;
    use fancy_regex::Regex;
    use std::sync::LazyLock;

    static RE_LEADING: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)(?P<beginsp>^\s+)").expect("error with leading space regex")
    });
    static RE_TRAILING: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)(?P<endsp>\s+$)").expect("error with trailing space regex")
    });

    let mut buf = text.to_owned();

    if !lead.is_empty() {
        buf = RE_LEADING
            .replace_all(&buf, |cap: &Captures| {
                let spaces = cap.get(1).expect("valid").as_str();
                format!("{}{}{}", lead.get_prefix(), spaces, lead.get_suffix())
            })
            .into_owned();
    }

    if !trail.is_empty() {
        buf = RE_TRAILING
            .replace_all(&buf, |cap: &Captures| {
                let spaces = cap.get(1).expect("valid").as_str();
                format!("{}{}{}", trail.get_prefix(), spaces, trail.get_suffix())
            })
            .into_owned();
    }

    buf
}

pub fn convert_style(style: nu_ansi_term::Style) -> Color {
    Color::new(style.prefix().to_string(), style.suffix().to_string())
}

pub fn is_color_empty(c: &Color) -> bool {
    c.get_prefix().is_empty() && c.get_suffix().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_charset_preserves_color_sequences() {
        // SGR color sequences (ending with 'm') should be preserved since
        // Nushell uses them for LS_COLORS and internal styling.
        let input = "\x1b[31mred text\x1b[0m";
        let result = clean_charset(input);
        assert_eq!(result, "\x1b[31mred text\x1b[0m");
    }

    #[test]
    fn clean_charset_strips_cursor_movement() {
        // Cursor movement: ESC[H (cursor home) — dangerous, should be stripped
        let input = "\x1b[Hmalicious content";
        let result = clean_charset(input);
        assert_eq!(result, "malicious content");
    }

    #[test]
    fn clean_charset_strips_screen_clear() {
        // Screen clear: ESC[2J — dangerous, should be stripped
        let input = "\x1b[2Jmalicious content";
        let result = clean_charset(input);
        assert_eq!(result, "malicious content");
    }

    #[test]
    fn clean_charset_strips_cursor_up() {
        // Cursor up: ESC[5A — dangerous, should be stripped
        let input = "visible\x1b[5Ahidden overwrite";
        let result = clean_charset(input);
        assert_eq!(result, "visiblehidden overwrite");
    }

    #[test]
    fn clean_charset_strips_osc_sequences() {
        // OSC title change: ESC]0;title BEL — dangerous, should be stripped
        let input = "\x1b]0;evil title\x07normal text";
        let result = clean_charset(input);
        assert_eq!(result, "normal text");
    }

    #[test]
    fn clean_charset_preserves_plain_text() {
        let input = "hello world";
        let result = clean_charset(input);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn clean_charset_still_converts_tabs_to_spaces() {
        let input = "col1\tcol2";
        let result = clean_charset(input);
        assert_eq!(result, "col1    col2");
    }

    #[test]
    fn clean_charset_still_removes_carriage_returns() {
        let input = "line1\r\nline2";
        let result = clean_charset(input);
        assert_eq!(result, "line1\nline2");
    }

    #[test]
    fn clean_charset_strips_dangerous_preserves_color_mixed() {
        // Mix of color (safe) and cursor movement (dangerous)
        let input = "\x1b[31mred\x1b[0m\x1b[2J\x1b[Hevil";
        let result = clean_charset(input);
        assert_eq!(result, "\x1b[31mred\x1b[0mevil");
    }

    #[test]
    fn clean_charset_handles_empty_string() {
        let result = clean_charset("");
        assert_eq!(result, "");
    }

    #[test]
    fn clean_charset_exhaustive_csi_filter() {
        // Loop through all CSI final bytes to prove exactly which are kept vs stripped.
        // CSI sequences end with an ASCII letter or '@' or '`'.
        // Only 'm' (SGR — colors/styles) should be preserved.
        let csi_final_bytes: Vec<char> = ('@'..='~').collect(); // ASCII 0x40..0x7E

        for &final_byte in &csi_final_bytes {
            let input = format!("before\x1b[1{final_byte}after");
            let result = clean_charset(&input);

            if final_byte == 'm' {
                assert_eq!(
                    result, input,
                    "SGR sequence (final byte '{final_byte}') should be PRESERVED"
                );
            } else {
                assert_eq!(
                    result, "beforeafter",
                    "CSI sequence (final byte '{final_byte}') should be STRIPPED"
                );
            }
        }
    }

    #[test]
    fn clean_charset_exhaustive_dangerous_sequences() {
        // Comprehensive list of known dangerous escape sequences and their categories.
        let stripped: Vec<(&str, &str)> = vec![
            // Cursor movement
            ("\x1b[A", "Cursor Up"),
            ("\x1b[B", "Cursor Down"),
            ("\x1b[C", "Cursor Forward"),
            ("\x1b[D", "Cursor Back"),
            ("\x1b[E", "Cursor Next Line"),
            ("\x1b[F", "Cursor Previous Line"),
            ("\x1b[G", "Cursor Horizontal Absolute"),
            ("\x1b[H", "Cursor Position"),
            ("\x1b[5A", "Cursor Up 5"),
            ("\x1b[10;20H", "Cursor to row 10 col 20"),
            // Erase
            ("\x1b[J", "Erase in Display"),
            ("\x1b[2J", "Erase Entire Screen"),
            ("\x1b[K", "Erase in Line"),
            ("\x1b[2K", "Erase Entire Line"),
            // Scroll
            ("\x1b[S", "Scroll Up"),
            ("\x1b[T", "Scroll Down"),
            ("\x1b[3S", "Scroll Up 3"),
            // Other CSI
            ("\x1b[6n", "Device Status Report"),
            ("\x1b[s", "Save Cursor Position"),
            ("\x1b[u", "Restore Cursor Position"),
            ("\x1b[?25l", "Hide Cursor"),
            ("\x1b[?25h", "Show Cursor"),
            ("\x1b[L", "Insert Lines"),
            ("\x1b[P", "Delete Characters"),
            ("\x1b[@", "Insert Characters"),
            // OSC sequences
            ("\x1b]0;title\x07", "Set Window Title (BEL)"),
            ("\x1b]0;title\x1b\\", "Set Window Title (ST)"),
            ("\x1b]52;c;data\x07", "Clipboard Write (BEL)"),
            ("\x1b]52;c;data\x1b\\", "Clipboard Write (ST)"),
            ("\x1b]8;;http://evil.com\x07", "Hyperlink (BEL)"),
            // Two-byte ESC sequences
            ("\x1b7", "Save Cursor (DEC)"),
            ("\x1b8", "Restore Cursor (DEC)"),
            ("\x1bD", "Index"),
            ("\x1bM", "Reverse Index"),
            ("\x1bc", "Reset Terminal (RIS)"),
        ];

        for (seq, name) in &stripped {
            let input = format!("before{seq}after");
            let result = clean_charset(&input);
            assert_eq!(result, "beforeafter", "{name} ({seq:?}) should be STRIPPED");
        }

        // SGR sequences that MUST be preserved
        let preserved: Vec<(&str, &str)> = vec![
            ("\x1b[0m", "Reset"),
            ("\x1b[1m", "Bold"),
            ("\x1b[2m", "Dim"),
            ("\x1b[3m", "Italic"),
            ("\x1b[4m", "Underline"),
            ("\x1b[7m", "Reverse"),
            ("\x1b[9m", "Strikethrough"),
            ("\x1b[31m", "Red Foreground"),
            ("\x1b[42m", "Green Background"),
            ("\x1b[38;5;196m", "256-color Red"),
            ("\x1b[48;5;21m", "256-color Blue Background"),
            ("\x1b[38;2;255;0;0m", "24-bit RGB Red"),
            ("\x1b[1;31;42m", "Bold + Red FG + Green BG"),
        ];

        for (seq, name) in &preserved {
            let input = format!("before{seq}after");
            let result = clean_charset(&input);
            assert_eq!(result, input, "{name} ({seq:?}) should be PRESERVED");
        }
    }
}
