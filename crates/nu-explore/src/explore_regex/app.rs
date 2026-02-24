//! Application state and logic for the regex explorer.

use crate::explore_regex::colors;
use crate::explore_regex::quick_ref::{QuickRefEntry, get_flattened_entries};
use edtui::{EditorMode, EditorState, Lines, actions::InsertChar};
use fancy_regex::Regex;
use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};

/// Which pane currently has input focus.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum InputFocus {
    #[default]
    Regex,
    Sample,
    QuickRef,
}

/// Main application state for the regex explorer.
pub struct App {
    /// Which input pane currently has focus (Regex, Sample, or QuickRef)
    pub input_focus: InputFocus,
    /// Editor state for the regex pattern input (single-line)
    pub regex_input: EditorState,
    /// Editor state for the test string input (multi-line)
    pub sample_text: EditorState,
    /// Compiled regex pattern, if valid
    pub compiled_regex: Option<Regex>,
    /// Error message from regex compilation, if invalid
    pub regex_error: Option<String>,
    /// Vertical scroll offset for the sample text viewport (in lines)
    pub sample_scroll_v: u16,
    /// Horizontal scroll offset for the sample text viewport (in characters)
    pub sample_scroll_h: u16,
    /// Height of the visible sample text viewport (updated each frame)
    pub sample_view_height: u16,
    /// Number of regex matches found in the sample text
    pub match_count: usize,
    // Quick reference panel state
    /// Whether the quick reference panel is currently visible
    pub show_quick_ref: bool,
    /// Whether the help modal is currently visible
    pub show_help: bool,
    /// Index of the currently selected quick reference entry
    pub quick_ref_selected: usize,
    /// Vertical scroll position in the quick reference panel
    pub quick_ref_scroll: usize,
    /// Horizontal scroll position in the quick reference panel
    pub quick_ref_scroll_h: u16,
    /// Height of the quick reference viewport (updated each frame)
    pub quick_ref_view_height: usize,
    /// Width of the quick reference viewport (updated each frame)
    pub quick_ref_view_width: u16,
    /// Flattened list of all quick reference entries
    pub quick_ref_entries: Vec<QuickRefEntry>,
}

impl App {
    pub fn new(input_string: String) -> Self {
        let mut regex_input = EditorState::default();
        regex_input.mode = EditorMode::Insert; // Enable modeless editing

        let mut sample_text = EditorState::new(Lines::from(input_string.as_str()));
        sample_text.mode = EditorMode::Insert; // Enable modeless editing

        let entries = get_flattened_entries();
        let initial_selected = Self::find_next_item(&entries, 0).unwrap_or(0);

        Self {
            input_focus: InputFocus::default(),
            regex_input,
            sample_text,
            compiled_regex: None,
            regex_error: None,
            sample_scroll_v: 0,
            sample_scroll_h: 0,
            sample_view_height: 0,
            match_count: 0,
            show_quick_ref: false,
            show_help: false,
            quick_ref_selected: initial_selected,
            quick_ref_scroll: 0,
            quick_ref_scroll_h: 0,
            quick_ref_view_height: 0,
            quick_ref_view_width: 0,
            quick_ref_entries: entries,
        }
    }

    pub fn get_sample_text(&self) -> String {
        self.sample_text.lines.to_string()
    }

    pub fn get_regex_input(&self) -> String {
        self.regex_input.lines.to_string()
    }

    pub fn compile_regex(&mut self) {
        self.compiled_regex = None;
        self.regex_error = None;
        self.match_count = 0;

        let regex_input = self.regex_input.lines.to_string();

        if regex_input.is_empty() {
            return;
        }

        match Regex::new(&regex_input) {
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
    /// This is more efficient than `compile_regex()` when only the sample text changes.
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

    /// Toggle the help modal visibility.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
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
            // Insert each character of the item at the cursor position
            for ch in item.insert.chars() {
                self.regex_input.execute(InsertChar(ch));
            }
            self.compile_regex();
        }
    }

    /// Scroll quick reference panel left.
    pub fn quick_ref_scroll_left(&mut self) {
        self.quick_ref_scroll_h = self.quick_ref_scroll_h.saturating_sub(4);
    }

    /// Scroll quick reference panel right.
    pub fn quick_ref_scroll_right(&mut self) {
        self.quick_ref_scroll_h = self.quick_ref_scroll_h.saturating_add(4);
    }

    /// Scroll quick reference panel to home (beginning of line).
    pub fn quick_ref_scroll_home(&mut self) {
        self.quick_ref_scroll_h = 0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explore_regex::quick_ref::QuickRefItem;

    /// Create a test entry list with known structure:
    /// [Category, Item, Item, Category, Item, Category, Item, Item]
    fn test_entries() -> Vec<QuickRefEntry> {
        vec![
            QuickRefEntry::Category("Cat1"),
            QuickRefEntry::Item(QuickRefItem {
                syntax: "a",
                description: "desc a",
                insert: "a",
            }),
            QuickRefEntry::Item(QuickRefItem {
                syntax: "b",
                description: "desc b",
                insert: "b",
            }),
            QuickRefEntry::Category("Cat2"),
            QuickRefEntry::Item(QuickRefItem {
                syntax: "c",
                description: "desc c",
                insert: "c",
            }),
            QuickRefEntry::Category("Cat3"),
            QuickRefEntry::Item(QuickRefItem {
                syntax: "d",
                description: "desc d",
                insert: "d",
            }),
            QuickRefEntry::Item(QuickRefItem {
                syntax: "e",
                description: "desc e",
                insert: "e",
            }),
        ]
    }

    // ─── is_selectable tests ─────────────────────────────────────────────────

    #[test]
    fn test_is_selectable_item() {
        let entries = test_entries();
        assert!(App::is_selectable(&entries, 1)); // Item "a"
        assert!(App::is_selectable(&entries, 2)); // Item "b"
        assert!(App::is_selectable(&entries, 4)); // Item "c"
    }

    #[test]
    fn test_is_selectable_category() {
        let entries = test_entries();
        assert!(!App::is_selectable(&entries, 0)); // Category "Cat1"
        assert!(!App::is_selectable(&entries, 3)); // Category "Cat2"
        assert!(!App::is_selectable(&entries, 5)); // Category "Cat3"
    }

    #[test]
    fn test_is_selectable_out_of_bounds() {
        let entries = test_entries();
        assert!(!App::is_selectable(&entries, 100));
    }

    #[test]
    fn test_is_selectable_empty_list() {
        let entries: Vec<QuickRefEntry> = vec![];
        assert!(!App::is_selectable(&entries, 0));
    }

    // ─── find_next_item tests ────────────────────────────────────────────────

    #[test]
    fn test_find_next_item_basic() {
        let entries = test_entries();
        // From item 1, next item is 2
        assert_eq!(App::find_next_item(&entries, 1), Some(2));
    }

    #[test]
    fn test_find_next_item_skips_category() {
        let entries = test_entries();
        // From item 2, next is item 4 (skips category at 3)
        assert_eq!(App::find_next_item(&entries, 2), Some(4));
    }

    #[test]
    fn test_find_next_item_from_category() {
        let entries = test_entries();
        // From category 0, next item is 1
        assert_eq!(App::find_next_item(&entries, 0), Some(1));
        // From category 3, next item is 4
        assert_eq!(App::find_next_item(&entries, 3), Some(4));
    }

    #[test]
    fn test_find_next_item_at_end() {
        let entries = test_entries();
        // From last item (7), no next item
        assert_eq!(App::find_next_item(&entries, 7), None);
    }

    #[test]
    fn test_find_next_item_empty_list() {
        let entries: Vec<QuickRefEntry> = vec![];
        assert_eq!(App::find_next_item(&entries, 0), None);
    }

    // ─── find_prev_item tests ────────────────────────────────────────────────

    #[test]
    fn test_find_prev_item_basic() {
        let entries = test_entries();
        // From item 2, prev item is 1
        assert_eq!(App::find_prev_item(&entries, 2), Some(1));
    }

    #[test]
    fn test_find_prev_item_skips_category() {
        let entries = test_entries();
        // From item 4, prev is item 2 (skips category at 3)
        assert_eq!(App::find_prev_item(&entries, 4), Some(2));
    }

    #[test]
    fn test_find_prev_item_from_category() {
        let entries = test_entries();
        // From category 3, prev item is 2
        assert_eq!(App::find_prev_item(&entries, 3), Some(2));
        // From category 5, prev item is 4
        assert_eq!(App::find_prev_item(&entries, 5), Some(4));
    }

    #[test]
    fn test_find_prev_item_at_start() {
        let entries = test_entries();
        // From first item (1), no prev item (0 is a category)
        assert_eq!(App::find_prev_item(&entries, 1), None);
        // From index 0, no prev item
        assert_eq!(App::find_prev_item(&entries, 0), None);
    }

    #[test]
    fn test_find_prev_item_empty_list() {
        let entries: Vec<QuickRefEntry> = vec![];
        assert_eq!(App::find_prev_item(&entries, 0), None);
    }

    // ─── find_nearest_item tests ─────────────────────────────────────────────

    #[test]
    fn test_find_nearest_item_on_item() {
        let entries = test_entries();
        // On item 1, nearest is itself
        assert_eq!(App::find_nearest_item(&entries, 1), Some(1));
    }

    #[test]
    fn test_find_nearest_item_on_category() {
        let entries = test_entries();
        // On category 0, nearest forward is item 1
        assert_eq!(App::find_nearest_item(&entries, 0), Some(1));
        // On category 3, nearest forward is item 4
        assert_eq!(App::find_nearest_item(&entries, 3), Some(4));
    }

    #[test]
    fn test_find_nearest_item_past_end() {
        let entries = test_entries();
        assert_eq!(App::find_nearest_item(&entries, 100), None);
    }

    #[test]
    fn test_find_nearest_item_empty_list() {
        let entries: Vec<QuickRefEntry> = vec![];
        assert_eq!(App::find_nearest_item(&entries, 0), None);
    }

    // ─── find_nearest_item_reverse tests ─────────────────────────────────────

    #[test]
    fn test_find_nearest_item_reverse_on_item() {
        let entries = test_entries();
        // On item 4, nearest reverse is itself
        assert_eq!(App::find_nearest_item_reverse(&entries, 4), Some(4));
    }

    #[test]
    fn test_find_nearest_item_reverse_on_category() {
        let entries = test_entries();
        // On category 3, nearest reverse is item 2
        assert_eq!(App::find_nearest_item_reverse(&entries, 3), Some(2));
        // On category 5, nearest reverse is item 4
        assert_eq!(App::find_nearest_item_reverse(&entries, 5), Some(4));
    }

    #[test]
    fn test_find_nearest_item_reverse_at_start_category() {
        let entries = test_entries();
        // On category 0, no item before or at
        assert_eq!(App::find_nearest_item_reverse(&entries, 0), None);
    }

    #[test]
    fn test_find_nearest_item_reverse_empty_list() {
        let entries: Vec<QuickRefEntry> = vec![];
        assert_eq!(App::find_nearest_item_reverse(&entries, 0), None);
    }

    // ─── Navigation method tests (quick_ref_up/down) ─────────────────────────

    #[test]
    fn test_quick_ref_down_moves_to_next_item() {
        let mut app = create_test_app();
        app.quick_ref_selected = 1; // Start at item "a"

        app.quick_ref_down();

        assert_eq!(app.quick_ref_selected, 2); // Moved to item "b"
    }

    #[test]
    fn test_quick_ref_down_skips_category() {
        let mut app = create_test_app();
        app.quick_ref_selected = 2; // Start at item "b"

        app.quick_ref_down();

        assert_eq!(app.quick_ref_selected, 4); // Skipped category, moved to item "c"
    }

    #[test]
    fn test_quick_ref_down_stays_at_last_item() {
        let mut app = create_test_app();
        app.quick_ref_selected = 7; // Start at last item "e"

        app.quick_ref_down();

        assert_eq!(app.quick_ref_selected, 7); // Stayed at last item
    }

    #[test]
    fn test_quick_ref_up_moves_to_prev_item() {
        let mut app = create_test_app();
        app.quick_ref_selected = 2; // Start at item "b"

        app.quick_ref_up();

        assert_eq!(app.quick_ref_selected, 1); // Moved to item "a"
    }

    #[test]
    fn test_quick_ref_up_skips_category() {
        let mut app = create_test_app();
        app.quick_ref_selected = 4; // Start at item "c"

        app.quick_ref_up();

        assert_eq!(app.quick_ref_selected, 2); // Skipped category, moved to item "b"
    }

    #[test]
    fn test_quick_ref_up_stays_at_first_item() {
        let mut app = create_test_app();
        app.quick_ref_selected = 1; // Start at first item "a"

        app.quick_ref_up();

        assert_eq!(app.quick_ref_selected, 1); // Stayed at first item
    }

    // ─── Page navigation tests ───────────────────────────────────────────────

    #[test]
    fn test_quick_ref_page_down() {
        let mut app = create_test_app();
        app.quick_ref_selected = 1; // Start at item "a"
        app.quick_ref_view_height = 3; // Page size of 3

        app.quick_ref_page_down();

        // Target would be index 4, which is item "c"
        assert_eq!(app.quick_ref_selected, 4);
    }

    #[test]
    fn test_quick_ref_page_down_lands_on_category() {
        let mut app = create_test_app();
        app.quick_ref_selected = 1; // Start at item "a"
        app.quick_ref_view_height = 2; // Page size of 2

        app.quick_ref_page_down();

        // Target would be index 3 (category), should find nearest item at or before: 2
        assert_eq!(app.quick_ref_selected, 2);
    }

    #[test]
    fn test_quick_ref_page_down_at_end() {
        let mut app = create_test_app();
        app.quick_ref_selected = 7; // Start at last item "e"
        app.quick_ref_view_height = 3;

        app.quick_ref_page_down();

        // Should stay at last item
        assert_eq!(app.quick_ref_selected, 7);
    }

    #[test]
    fn test_quick_ref_page_up() {
        let mut app = create_test_app();
        app.quick_ref_selected = 7; // Start at item "e"
        app.quick_ref_view_height = 3; // Page size of 3

        app.quick_ref_page_up();

        // Target would be index 4, which is item "c"
        assert_eq!(app.quick_ref_selected, 4);
    }

    #[test]
    fn test_quick_ref_page_up_lands_on_category() {
        let mut app = create_test_app();
        app.quick_ref_selected = 6; // Start at item "d"
        app.quick_ref_view_height = 3; // Page size of 3

        app.quick_ref_page_up();

        // Target would be index 3 (category), should find nearest item at or after: 4
        assert_eq!(app.quick_ref_selected, 4);
    }

    #[test]
    fn test_quick_ref_page_up_at_start() {
        let mut app = create_test_app();
        app.quick_ref_selected = 1; // Start at first item "a"
        app.quick_ref_view_height = 3;

        app.quick_ref_page_up();

        // Should stay at first item
        assert_eq!(app.quick_ref_selected, 1);
    }

    #[test]
    fn test_quick_ref_page_navigation_with_zero_height() {
        let mut app = create_test_app();
        app.quick_ref_selected = 2;
        app.quick_ref_view_height = 0; // Edge case: zero height

        app.quick_ref_page_down();

        // Should use height of 1, so move by 1
        // From 2, target is 3 (category), nearest at or before is 2
        assert_eq!(app.quick_ref_selected, 2);
    }

    // ─── Horizontal scroll tests ─────────────────────────────────────────────

    #[test]
    fn test_quick_ref_scroll_right() {
        let mut app = create_test_app();
        assert_eq!(app.quick_ref_scroll_h, 0);

        app.quick_ref_scroll_right();
        assert_eq!(app.quick_ref_scroll_h, 4);

        app.quick_ref_scroll_right();
        assert_eq!(app.quick_ref_scroll_h, 8);
    }

    #[test]
    fn test_quick_ref_scroll_left() {
        let mut app = create_test_app();
        app.quick_ref_scroll_h = 8;

        app.quick_ref_scroll_left();
        assert_eq!(app.quick_ref_scroll_h, 4);

        app.quick_ref_scroll_left();
        assert_eq!(app.quick_ref_scroll_h, 0);
    }

    #[test]
    fn test_quick_ref_scroll_left_at_zero() {
        let mut app = create_test_app();
        assert_eq!(app.quick_ref_scroll_h, 0);

        app.quick_ref_scroll_left();
        assert_eq!(app.quick_ref_scroll_h, 0); // Should not underflow
    }

    #[test]
    fn test_quick_ref_scroll_home() {
        let mut app = create_test_app();
        app.quick_ref_scroll_h = 20;

        app.quick_ref_scroll_home();
        assert_eq!(app.quick_ref_scroll_h, 0);
    }

    #[test]
    fn test_quick_ref_scroll_home_already_at_zero() {
        let mut app = create_test_app();
        assert_eq!(app.quick_ref_scroll_h, 0);

        app.quick_ref_scroll_home();
        assert_eq!(app.quick_ref_scroll_h, 0);
    }

    // ─── Unicode handling tests ───────────────────────────────────────────────

    #[test]
    fn test_compile_regex_with_unicode_pattern() {
        let mut app = App::new(String::new());
        app.regex_input = EditorState::new(Lines::from("\\p{L}+"));
        app.compile_regex();
        assert!(app.compiled_regex.is_some());
        assert!(app.regex_error.is_none());
    }

    #[test]
    fn test_get_highlighted_text_with_unicode_sample() {
        let mut app = App::new("12345abcde項目".to_string());
        app.regex_input = EditorState::new(Lines::from("\\w+"));
        app.compile_regex();
        let highlighted = app.get_highlighted_text();
        // Should not panic and should produce some text
        assert!(!highlighted.lines.is_empty());
    }

    // ─── Helper ──────────────────────────────────────────────────────────────

    fn create_test_app() -> App {
        let mut app = App::new(String::new());
        app.quick_ref_entries = test_entries();
        app.quick_ref_selected = 1; // Default to first item
        app.quick_ref_view_height = 5;
        app
    }
}
