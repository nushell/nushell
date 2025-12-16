//! Application state and logic for the regex explorer.

use crate::explore_regex::colors;
use crate::explore_regex::quick_ref::{QuickRefEntry, get_flattened_entries};
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
    QuickRef,
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
    // Quick reference panel state
    pub show_quick_ref: bool,
    pub quick_ref_selected: usize,
    pub quick_ref_scroll: usize,
    pub quick_ref_view_height: usize,
    pub quick_ref_entries: Vec<QuickRefEntry>,
}

impl<'a> App<'a> {
    pub fn new(input_string: String) -> Self {
        let mut regex_textarea = TextArea::default();
        regex_textarea.set_cursor_line_style(Style::new());

        let mut sample_textarea = TextArea::default();
        sample_textarea.insert_str(&input_string);
        sample_textarea.set_cursor_line_style(Style::new());
        sample_textarea.move_cursor(CursorMove::Top);

        // Initialize with first selectable item (skip first category header)
        let entries = get_flattened_entries();
        let initial_selected = if entries.len() > 1 { 1 } else { 0 };

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
            show_quick_ref: false,
            quick_ref_selected: initial_selected,
            quick_ref_scroll: 0,
            quick_ref_view_height: 0,
            quick_ref_entries: entries,
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

    /// Toggle the quick reference panel visibility
    pub fn toggle_quick_ref(&mut self) {
        self.show_quick_ref = !self.show_quick_ref;
        if self.show_quick_ref {
            self.input_focus = InputFocus::QuickRef;
        } else {
            self.input_focus = InputFocus::Regex;
        }
    }

    /// Move selection up in the quick reference list, skipping category headers
    pub fn quick_ref_up(&mut self) {
        if self.quick_ref_selected > 0 {
            self.quick_ref_selected -= 1;
            // Skip category headers when moving up
            while self.quick_ref_selected > 0 {
                if let QuickRefEntry::Category(_) = &self.quick_ref_entries[self.quick_ref_selected]
                {
                    self.quick_ref_selected -= 1;
                } else {
                    break;
                }
            }
            // If we landed on a category header at position 0, move to first item
            if let QuickRefEntry::Category(_) = &self.quick_ref_entries[self.quick_ref_selected] {
                self.quick_ref_selected += 1;
            }
        }
    }

    /// Move selection down in the quick reference list, skipping category headers
    pub fn quick_ref_down(&mut self) {
        if self.quick_ref_selected < self.quick_ref_entries.len() - 1 {
            self.quick_ref_selected += 1;
            // Skip category headers when moving down
            while self.quick_ref_selected < self.quick_ref_entries.len() - 1 {
                if let QuickRefEntry::Category(_) = &self.quick_ref_entries[self.quick_ref_selected]
                {
                    self.quick_ref_selected += 1;
                } else {
                    break;
                }
            }
        }
    }

    /// Move selection up by one page in the quick reference list
    pub fn quick_ref_page_up(&mut self) {
        let page_size = self.quick_ref_view_height.max(1);
        for _ in 0..page_size {
            if self.quick_ref_selected <= 1 {
                break;
            }
            self.quick_ref_up();
        }
    }

    /// Move selection down by one page in the quick reference list
    pub fn quick_ref_page_down(&mut self) {
        let page_size = self.quick_ref_view_height.max(1);
        for _ in 0..page_size {
            if self.quick_ref_selected >= self.quick_ref_entries.len() - 1 {
                break;
            }
            self.quick_ref_down();
        }
    }

    /// Insert the selected quick reference item into the regex input
    pub fn insert_selected_quick_ref(&mut self) {
        if let Some(QuickRefEntry::Item(item)) = self.quick_ref_entries.get(self.quick_ref_selected)
        {
            self.regex_textarea.insert_str(item.insert);
            self.compile_regex();
        }
    }
}
