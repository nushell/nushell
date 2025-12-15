//! Application state and logic for the regex explorer.

use crate::explore_regex::colors;
use fancy_regex::Regex;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use tui_textarea::{CursorMove, TextArea};

#[derive(Default)]
pub enum InputFocus {
    #[default]
    Regex,
    Sample,
}

pub struct App<'a> {
    pub input_focus: InputFocus,
    pub regex_textarea: TextArea<'a>,
    pub sample_textarea: TextArea<'a>,
    pub compiled_regex: Option<Regex>,
    pub regex_error: Option<String>,
    pub sample_scroll_v: u16,
    pub sample_scroll_h: u16,
    pub sample_view_height: u16,
    pub match_count: usize,
}

impl<'a> App<'a> {
    pub fn new(input_string: String) -> Self {
        let mut regex_textarea = TextArea::default();
        regex_textarea.set_cursor_line_style(Style::new());

        let mut sample_textarea = TextArea::default();
        sample_textarea.insert_str(&input_string);
        sample_textarea.set_cursor_line_style(Style::new());
        sample_textarea.move_cursor(CursorMove::Top);

        Self {
            input_focus: InputFocus::default(),
            regex_textarea,
            sample_textarea,
            compiled_regex: None,
            regex_error: None,
            sample_scroll_v: 0,
            sample_scroll_h: 0,
            sample_view_height: 0,
            match_count: 0,
        }
    }

    pub fn get_sample_text(&self) -> String {
        self.sample_textarea.lines().join("\n")
    }

    pub fn get_regex_input(&self) -> String {
        self.regex_textarea
            .lines()
            .first()
            .cloned()
            .unwrap_or_default()
    }

    pub fn compile_regex(&mut self) {
        self.compiled_regex = None;
        self.regex_error = None;
        self.match_count = 0;

        let Some(regex_input) = self.regex_textarea.lines().first() else {
            return;
        };

        if regex_input.is_empty() {
            return;
        }

        match Regex::new(regex_input) {
            Ok(regex) => {
                self.compiled_regex = Some(regex);
                self.update_match_count();
            }
            Err(e) => {
                self.regex_error = Some(format!("{e}"));
            }
        }
    }

    /// Update match count using the already-compiled regex.
    /// This is more efficient than compile_regex() when only the sample text changes.
    pub fn update_match_count(&mut self) {
        if let Some(ref regex) = self.compiled_regex {
            let sample_text = self.get_sample_text();
            self.match_count = regex.captures_iter(&sample_text).flatten().count();
        }
    }

    pub fn get_highlighted_text(&self) -> Text<'static> {
        let sample_text = self.get_sample_text();
        let Some(regex) = &self.compiled_regex else {
            return Text::from(sample_text);
        };

        let highlight_styles = [
            Style::new().bg(colors::HIGHLIGHT_1).fg(Color::Black),
            Style::new().bg(colors::HIGHLIGHT_2).fg(Color::Black),
            Style::new().bg(colors::HIGHLIGHT_3).fg(Color::Black),
            Style::new().bg(colors::HIGHLIGHT_4).fg(Color::Black),
            Style::new().bg(colors::HIGHLIGHT_5).fg(Color::Black),
            Style::new().bg(colors::HIGHLIGHT_6).fg(Color::Black),
            Style::new().bg(colors::HIGHLIGHT_7).fg(Color::White),
            Style::new().bg(colors::HIGHLIGHT_8).fg(Color::Black),
            Style::new().bg(colors::HIGHLIGHT_9).fg(Color::White),
        ];

        let mut highlights: Vec<(usize, usize, Style)> = Vec::new();
        for capture in regex.captures_iter(&sample_text).flatten() {
            for (group, submatch) in capture.iter().enumerate() {
                if let Some(submatch) = submatch {
                    highlights.push((
                        submatch.start(),
                        submatch.end(),
                        highlight_styles[group % highlight_styles.len()],
                    ));
                }
            }
        }

        // This is a fallback style when a span has no highlight match. Although,
        // to make sure full matches from not being highlighted, we need to make sure
        // this fallback is the last element, even after the sort later.
        highlights.push((0, sample_text.len(), Style::new()));

        // Sort the highlights by their size and start position. This lets us
        // to exit as soon as one overlapping match is found below.
        highlights.sort_by(|a, b| ((a.1 - a.0), a.0).cmp(&(b.1 - b.0, b.0)));

        // All the boundary points in the vector.
        let mut boundaries: Vec<usize> = highlights.iter().flat_map(|(s, e, _)| [*s, *e]).collect();

        boundaries.sort();
        boundaries.dedup();

        // \n in Spans are ignored. Therefore, we need to construct them ourselves.
        let mut lines: Vec<Line> = Vec::new();
        let mut current_line = Line::from("");

        // Generate styled spans as necessary.
        // TODO: Do this in a more efficient way. You can flatten the matches using
        // a stack and last known position instead of a nested lookup here.
        for window in boundaries.windows(2) {
            if let [s, e] = window
                && let Some((_, _, style)) =
                    highlights.iter().find(|(c_s, c_e, _)| c_s <= s && c_e >= e)
            {
                let fragment = &sample_text[*s..*e];
                for (idx, part) in fragment.split('\n').enumerate() {
                    // This works because usually, lines are terminated with a newline, therefore,
                    // we need to prefer updating the existing line for the first split item. But,
                    // starting with the second item, we know that there was an explicit newline in
                    // between.
                    if idx > 0 {
                        lines.push(current_line);
                        current_line = Line::from("");
                    }

                    if !part.is_empty() {
                        current_line.push_span(Span::styled(part.to_string(), *style));
                    }
                }
            }
        }

        lines.push(current_line);
        Text::from(lines)
    }
}
