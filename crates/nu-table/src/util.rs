use tabled::{
    builder::Builder,
    grid::util::string::string_width_multiline,
    settings::{width::Truncate, Modify, Padding, Style, Width},
};

pub fn string_width(text: &str) -> usize {
    string_width_multiline(text)
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
        .with(Modify::new((0, 0)).with(wrap))
        .to_string()
}

pub fn string_truncate(text: &str, width: usize) -> String {
    // todo: change me...

    let line = match text.lines().next() {
        Some(first_line) => first_line,
        None => return String::new(),
    };

    Truncate::truncate_text(line, width).into_owned()
}

pub fn clean_charset(text: &str) -> String {
    // todo: optimize, I bet it can be done in 1 path
    text.replace('\t', "    ").replace('\r', "")
}
