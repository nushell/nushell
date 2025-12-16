//! Application state and logic for the regex explorer.

use crate::explore_regex::colors;
use crate::explore_regex::quick_ref::{QuickRefEntry, get_flattened_entries};
use fancy_regex::Regex;
use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};
use tui_textarea::{CursorMove, TextArea};

/// Which pane currently has input focus.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum InputFocus {
    #[default]
    Regex,
    Sample,
    QuickRef,
}

/// Main application state for the regex explorer.
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

        let entries = get_flattened_entries();
        let initial_selected = Self::find_next_item(&entries, 0).unwrap_or(0);

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
                self.regex_error = Some(e.to_string());
            }
        }
    }

    /// Update match count using the already-compiled regex.
    /// More efficient than `compile_regex()` when only the sample text changes.
    pub fn update_match_count(&mut self) {
        if let Some(ref regex) = self.compiled_regex {
            let sample_text = self.get_sample_text();
            self.match_count = regex.captures_iter(&sample_text).flatten().count();
        }
    }

    /// Generate highlighted text with regex matches styled.
    pub fn get_highlighted_text(&self) -> Text<'static> {
        let sample_text = self.get_sample_text();
        let Some(regex) = &self.compiled_regex else {
            return Text::from(sample_text);
        };

        // Collect all match highlights: (start, end, style)
        let mut highlights: Vec<(usize, usize, Style)> = regex
            .captures_iter(&sample_text)
            .flatten()
            .flat_map(|capture| {
                capture
                    .iter()
                    .enumerate()
                    .filter_map(|(group, submatch)| {
                        submatch.map(|m| (m.start(), m.end(), colors::highlight_style(group)))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // Add fallback style for unhighlighted text (must be last after sort)
        highlights.push((0, sample_text.len(), Style::new()));

        // Sort by span size (smallest first) so inner groups take precedence
        highlights.sort_by_key(|(start, end, _)| (*end - *start, *start));

        // Collect all boundary points and deduplicate
        let mut boundaries: Vec<usize> = highlights.iter().flat_map(|(s, e, _)| [*s, *e]).collect();
        boundaries.sort();
        boundaries.dedup();

        // Build styled lines, handling newlines properly
        let mut lines: Vec<Line> = Vec::new();
        let mut current_line = Line::from("");

        for window in boundaries.windows(2) {
            let [start, end] = [window[0], window[1]];

            // Find the first (smallest) highlight that contains this range
            let Some((_, _, style)) = highlights.iter().find(|(s, e, _)| *s <= start && *e >= end)
            else {
                continue;
            };

            let fragment = &sample_text[start..end];
            for (idx, part) in fragment.split('\n').enumerate() {
                if idx > 0 {
                    lines.push(current_line);
                    current_line = Line::from("");
                }
                if !part.is_empty() {
                    current_line.push_span(Span::styled(part.to_string(), *style));
                }
            }
        }

        lines.push(current_line);
        Text::from(lines)
    }

    // ─── Quick Reference Panel ───────────────────────────────────────────

    /// Toggle the quick reference panel visibility.
    pub fn toggle_quick_ref(&mut self) {
        self.show_quick_ref = !self.show_quick_ref;
        self.input_focus = if self.show_quick_ref {
            InputFocus::QuickRef
        } else {
            InputFocus::Regex
        };
    }

    /// Close quick ref panel and return focus to regex input.
    pub fn close_quick_ref(&mut self) {
        self.show_quick_ref = false;
        self.input_focus = InputFocus::Regex;
    }

    /// Move selection up in the quick reference list.
    pub fn quick_ref_up(&mut self) {
        if let Some(prev) = Self::find_prev_item(&self.quick_ref_entries, self.quick_ref_selected) {
            self.quick_ref_selected = prev;
        }
    }

    /// Move selection down in the quick reference list.
    pub fn quick_ref_down(&mut self) {
        if let Some(next) = Self::find_next_item(&self.quick_ref_entries, self.quick_ref_selected) {
            self.quick_ref_selected = next;
        }
    }

    /// Move selection up by one page.
    pub fn quick_ref_page_up(&mut self) {
        let target = self
            .quick_ref_selected
            .saturating_sub(self.quick_ref_view_height.max(1));
        // Find the nearest selectable item at or after target
        self.quick_ref_selected = Self::find_nearest_item(&self.quick_ref_entries, target)
            .unwrap_or(self.quick_ref_selected);
    }

    /// Move selection down by one page.
    pub fn quick_ref_page_down(&mut self) {
        let target = (self.quick_ref_selected + self.quick_ref_view_height.max(1))
            .min(self.quick_ref_entries.len().saturating_sub(1));
        // Find the nearest selectable item at or before target
        self.quick_ref_selected = Self::find_nearest_item_reverse(&self.quick_ref_entries, target)
            .unwrap_or(self.quick_ref_selected);
    }

    /// Insert the selected quick reference item into the regex input.
    pub fn insert_selected_quick_ref(&mut self) {
        if let Some(QuickRefEntry::Item(item)) = self.quick_ref_entries.get(self.quick_ref_selected)
        {
            self.regex_textarea.insert_str(item.insert);
            self.compile_regex();
        }
    }

    /// Check if an entry at the given index is selectable (i.e., an Item, not a Category).
    fn is_selectable(entries: &[QuickRefEntry], idx: usize) -> bool {
        matches!(entries.get(idx), Some(QuickRefEntry::Item(_)))
    }

    /// Find the next selectable item after (not including) the given index.
    fn find_next_item(entries: &[QuickRefEntry], from: usize) -> Option<usize> {
        ((from + 1)..entries.len()).find(|&i| Self::is_selectable(entries, i))
    }

    /// Find the previous selectable item before (not including) the given index.
    fn find_prev_item(entries: &[QuickRefEntry], from: usize) -> Option<usize> {
        (0..from).rev().find(|&i| Self::is_selectable(entries, i))
    }

    /// Find the nearest selectable item at or after the given index.
    fn find_nearest_item(entries: &[QuickRefEntry], from: usize) -> Option<usize> {
        (from..entries.len()).find(|&i| Self::is_selectable(entries, i))
    }

    /// Find the nearest selectable item at or before the given index.
    fn find_nearest_item_reverse(entries: &[QuickRefEntry], from: usize) -> Option<usize> {
        (0..=from).rev().find(|&i| Self::is_selectable(entries, i))
    }
}
