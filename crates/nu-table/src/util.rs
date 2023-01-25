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
