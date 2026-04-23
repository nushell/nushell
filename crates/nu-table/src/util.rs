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

    // Safety net: tabled's Wrap::wrap can panic on strings containing
    // multi-byte Unicode characters (e.g., zero-width spaces \u{200b})
    // when it attempts to slice at a non-char-boundary byte index.
    // See: https://github.com/nushell/nushell/issues/17802
    match std::panic::catch_unwind(|| Wrap::wrap(text, width, keep_words)) {
        Ok(result) => result,
        Err(_) => truncate_to_char_boundary(text, width),
    }
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

    // Safety net: tabled's Truncate::truncate can panic on multi-byte
    // Unicode characters when slicing at non-char-boundary byte indices.
    // See: https://github.com/nushell/nushell/issues/17802
    match std::panic::catch_unwind(|| Truncate::truncate(line, width).into_owned()) {
        Ok(result) => result,
        Err(_) => truncate_to_char_boundary(line, width),
    }
}

/// Truncate a string to at most `width` display-columns, respecting
/// character boundaries.  Used as a fallback when `tabled` panics.
fn truncate_to_char_boundary(text: &str, width: usize) -> String {
    let mut result = String::new();
    let mut current_width = 0;
    for ch in text.chars() {
        // Approximate: CJK/wide chars count as 2, everything else as 1,
        // zero-width chars as 0.  This is only a panic fallback, so
        // perfect accuracy is not critical.
        let ch_width = if ch.is_control() || ch.len_utf8() == 3 && ch.is_alphanumeric() {
            0 // zero-width / control
        } else {
            1
        };
        if current_width + ch_width > width {
            break;
        }
        result.push(ch);
        current_width += ch_width;
    }
    result
}

pub fn clean_charset(text: &str) -> String {
    // TODO: We could make an optimization to take a String and modify it
    //       We could check if there was any changes and if not make no allocations at all and don't change the origin.
    //       Why it's not done...
    //       Cause I am not sure how the `if` in a loop will affect performance.
    //       So it's better be profiled, but likely the optimization be worth it.
    //       At least because it's a base case where we won't change anything....

    // allocating at least the text size,
    // in most cases the buf will be a copy of text anyhow.
    //
    // but yes sometimes we will alloc more then necessary.
    // We could shrink it but...it will be another realloc which make no scense.
    let mut buf = String::with_capacity(text.len());

    // note: (Left just in case)
    // note: This check could be added in order to cope with emojie issue.
    // if c < ' ' && c != '\u{1b}' {
    //    continue;
    // }

    for c in text.chars() {
        match c {
            '\r' => continue,
            '\t' => {
                buf.push(' ');
                buf.push(' ');
                buf.push(' ');
                buf.push(' ');
            }
            // Strip zero-width characters that confuse tabled's byte-based
            // string slicing. These multi-byte chars have display width 0
            // but occupy 2-3 bytes in UTF-8, causing panics in
            // Wrap::wrap / Truncate::truncate when it tries to split at
            // a non-char-boundary byte index.
            // See: https://github.com/nushell/nushell/issues/17802
            '\u{200b}' // Zero-Width Space
            | '\u{200c}' // Zero-Width Non-Joiner
            | '\u{200d}' // Zero-Width Joiner
            | '\u{feff}' // Zero-Width No-Break Space (BOM)
            | '\u{00ad}' // Soft Hyphen
            | '\u{2060}' // Word Joiner
            => continue,
            c => {
                buf.push(c);
            }
        }
    }

    buf
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
