use tabled::{builder::Builder, object::Cell, Modify, Padding, Style, Width};

pub fn string_width(text: &str) -> usize {
    tabled::papergrid::util::string_width_multiline_tab(text, 4)
}

pub fn string_wrap(text: &str, width: usize, keep_words: bool) -> String {
    // todo: change me...
    //
    // well... it's not efficient to build a table to wrap a string,
    // but ... it's better than a copy paste (is it?)

    if text.is_empty() {
        return String::new();
    }

    let wrap = if keep_words {
        Width::wrap(width).keep_words()
    } else {
        Width::wrap(width)
    };

    Builder::from_iter([[text]])
        .build()
        .with(Style::empty())
        .with(Padding::zero())
        .with(Modify::new(Cell(0, 0)).with(wrap))
        .to_string()
}

pub fn string_truncate(text: &str, width: usize) -> String {
    // todo: change me...

    let line = match text.lines().next() {
        Some(first_line) => first_line,
        None => return String::new(),
    };

    Builder::from_iter([[line]])
        .build()
        .with(Style::empty())
        .with(Padding::zero())
        .with(Width::truncate(width))
        .to_string()
}

// https://github.com/rust-lang/rust/blob/8a73f50d875840b8077b8ec080fa41881d7ce40d/compiler/rustc_errors/src/emitter.rs#L2477-L2497
const OUTPUT_REPLACEMENTS: &[(char, &str)] = &[
    ('\t', "    "),
    ('\r', ""),
    ('\u{200D}', ""),
    ('\u{202A}', ""),
    ('\u{202B}', ""),
    ('\u{202D}', ""),
    ('\u{202E}', ""),
    ('\u{2066}', ""),
    ('\u{2067}', ""),
    ('\u{2068}', ""),
    ('\u{202C}', ""),
    ('\u{2069}', ""),
];

pub fn normalize_whitespace(str: impl Into<String>) -> String {
    let mut s = str.into();
    for (c, replacement) in OUTPUT_REPLACEMENTS {
        s = s.replace(*c, replacement);
    }

    s
}
