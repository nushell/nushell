use tabled::{builder::Builder, Padding, Style, Width};

pub fn string_width(text: &str) -> usize {
    tabled::papergrid::util::string_width_multiline_tab(text, 4)
}

pub fn wrap_string(text: &str, width: usize) -> String {
    // todo: change me...
    //
    // well... it's not effitient to build a table to wrap a string,
    // but ... it's better than a copy paste (is it?)

    if text.is_empty() {
        return String::new();
    }

    Builder::from_iter([[text]])
        .build()
        .with(Padding::zero())
        .with(Style::empty())
        .with(Width::wrap(width))
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

pub fn string_wrap(text: &str, width: usize) -> String {
    // todo: change me...

    if text.is_empty() {
        return String::new();
    }

    Builder::from_iter([[text]])
        .build()
        .with(Style::empty())
        .with(Padding::zero())
        .with(Width::wrap(width))
        .to_string()
}
