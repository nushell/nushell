use reedline::{Completer, Editor, InputMode, Menu, MenuEvent, MenuSettings, Painter, Suggestion};
use std::sync::Arc;

/// Shared command line for menu source.
///
/// `Arc` is structural: reedline owns the menu and its completer in separate
/// boxes, so the recorded line must live in shared ownership (`Menu: Send`
/// rules out `Rc<RefCell>`). The lock never contends — reedline drives both
/// ends sequentially with `&mut` — so `parking_lot` keeps it infallible with
/// no poisoning branches.
#[derive(Clone, Default, Debug)]
pub struct MenuLine(Arc<parking_lot::Mutex<Option<String>>>);

impl MenuLine {
    /// Record current editor line.
    pub(crate) fn record(&self, line: &str) {
        *self.0.lock() = Some(line.to_owned());
    }

    /// Last recorded line.
    pub(crate) fn read(&self) -> Option<String> {
        self.0.lock().clone()
    }
}

/// How a source receives input and whether its layout is a list.
/// Reedline hands a fragment in Diff mode, so the source uses the recorded
/// line for context. Other input modes supply their own command text.
/// The wrapper and completer share this setting so they agree on the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceMode {
    Diff,
    /// List menus interpret selectors in the input they receive. Suggested
    /// spans must explicitly cover a selector to remove it on acceptance.
    List {
        input_mode: InputMode,
        suggested_span: bool,
    },
    Live,
}

impl SourceMode {
    pub(crate) fn of(mode: &InputMode) -> Self {
        match mode {
            InputMode::Diff => Self::Diff,
            _ => Self::Live,
        }
    }

    /// Whether the source uses the recorded line when it is available.
    pub(crate) fn is_diff(self) -> bool {
        matches!(
            self,
            Self::Diff
                | Self::List {
                    input_mode: InputMode::Diff,
                    ..
                }
        )
    }

    /// What the source parses, with a floored/clamped cursor.
    pub(crate) fn buffer(
        self,
        line: &MenuLine,
        handed: &str,
        pos: usize,
        replacing_start: usize,
    ) -> (String, usize) {
        let buffer = if self.is_diff() {
            line.read()
                .unwrap_or_else(|| " ".repeat(replacing_start) + handed)
        } else {
            handed.to_owned()
        };
        let cursor = buffer.floor_char_boundary(pos.min(buffer.len()));
        (buffer, cursor)
    }
}

/// Menu wrapper carrying the editor line to its source, and abandoning the menu when
/// the first fresh answer after activation is empty, so a cancelled picker (fzf Esc /
/// `input list` Esc) cannot leave an empty menu that relaunches on type.
pub struct SourcedMenu<M> {
    menu: M,
    line: Option<MenuLine>,
    mode: SourceMode,
    /// Whether the menu has shown values since it was last activated. Once it has,
    /// an empty answer means the typed text matches nothing, and the menu stays open
    /// to refill as the text changes (#19096).
    answered: bool,
    /// A sourced list gets its first rows during the update before painting.
    /// Until then, keep it active without showing its marker or cached rows.
    awaiting_list_answer: bool,
}

impl<M> SourcedMenu<M> {
    pub fn new(menu: M, line: MenuLine, mode: SourceMode) -> Self {
        Self {
            menu,
            line: Some(line),
            mode,
            answered: false,
            awaiting_list_answer: false,
        }
    }

    /// Engine-completer menus have no source line to carry, only abandon-on-empty.
    pub fn abandoning(menu: M) -> Self {
        Self {
            menu,
            line: None,
            mode: SourceMode::Live,
            answered: false,
            awaiting_list_answer: false,
        }
    }

    fn record(&self, editor: &Editor) {
        if let Some(line) = &self.line {
            line.record(editor.get_buffer());
        }
    }

    /// Close when the first fresh answer is empty; provisional/awaiting emptiness is
    /// still computing, and emptiness after values is a filter that matches nothing.
    fn abandon_if_empty(&mut self)
    where
        M: Menu,
    {
        if !self.get_values().is_empty() {
            self.answered = true;
        } else if self.menu.is_active()
            && !self.answered
            && !self.menu.results_are_provisional()
            && !self.is_awaiting_first_answer()
        {
            self.menu.menu_event(MenuEvent::Deactivate);
        }
    }
}

impl<M: Menu> Menu for SourcedMenu<M> {
    fn settings(&self) -> &MenuSettings {
        self.menu.settings()
    }

    fn name(&self) -> &str {
        self.menu.name()
    }

    fn indicator(&self) -> &str {
        self.menu.indicator()
    }

    fn is_active(&self) -> bool {
        self.menu.is_active()
    }

    fn set_active(&mut self, active: bool) {
        self.menu.set_active(active);
    }

    fn clear_input(&mut self) {
        self.menu.clear_input();
    }

    fn on_activate(&mut self) {
        self.menu.on_activate();
    }

    fn on_deactivate(&mut self) {
        self.menu.on_deactivate();
    }

    fn menu_event(&mut self, event: MenuEvent) {
        match event {
            MenuEvent::Activate(_) => {
                self.answered = false;
                self.awaiting_list_answer = matches!(self.mode, SourceMode::List { .. });
            }
            MenuEvent::Deactivate => self.awaiting_list_answer = false,
            _ => {}
        }
        self.menu.menu_event(event);
    }

    fn can_quick_complete(&self) -> bool {
        self.menu.can_quick_complete()
    }

    fn can_partially_complete(
        &mut self,
        values_updated: bool,
        editor: &mut Editor,
        completer: &mut dyn Completer,
    ) -> bool {
        self.record(editor);
        let spliced = self
            .menu
            .can_partially_complete(values_updated, editor, completer);
        if spliced && self.mode.is_diff() && self.line.is_some() {
            // The splice ran past this wrapper, so a `Diff` source would answer
            // with pre-splice spans without a refresh (#19053); anything else
            // already saw the spliced buffer through the handed line.
            self.update_values(editor, completer);
        } else {
            if spliced {
                self.record(editor);
            }
            // Reedline probes partial completion as a menu opens, and with quick
            // completions off that probe is the first fetch: a cancelled picker
            // answers empty here. Abandon now, before the prompt paints the menu
            // indicator; a probe that fetched nothing is still awaiting its answer.
            self.abandon_if_empty();
        }
        spliced
    }

    fn update_values(&mut self, editor: &mut Editor, completer: &mut dyn Completer) {
        self.record(editor);
        self.menu.update_values(editor, completer);
        self.awaiting_list_answer = false;
        self.abandon_if_empty();
    }

    fn reset_position(&mut self) {
        self.menu.reset_position();
    }

    fn reload(&mut self, updated: bool, editor: &mut Editor, completer: &mut dyn Completer) {
        self.record(editor);
        self.menu.reload(updated, editor, completer);
        if !updated {
            self.awaiting_list_answer = false;
        }
        self.abandon_if_empty();
    }

    fn update_working_details(
        &mut self,
        editor: &mut Editor,
        completer: &mut dyn Completer,
        painter: &Painter,
    ) {
        self.record(editor);
        self.menu.update_working_details(editor, completer, painter);
        self.awaiting_list_answer = false;
        self.abandon_if_empty();
    }

    fn replace_in_buffer(&self, editor: &mut Editor) {
        self.menu.replace_in_buffer(editor);
    }

    fn menu_required_lines(&self, terminal_columns: u16) -> u16 {
        self.menu.menu_required_lines(terminal_columns)
    }

    fn menu_string(&self, available_lines: u16, use_ansi_coloring: bool) -> String {
        self.menu.menu_string(available_lines, use_ansi_coloring)
    }

    fn min_rows(&self) -> u16 {
        self.menu.min_rows()
    }

    fn get_values(&self) -> &[Suggestion] {
        if self.awaiting_list_answer {
            &[]
        } else {
            self.menu.get_values()
        }
    }

    fn results_are_provisional(&self) -> bool {
        self.menu.results_are_provisional()
    }

    fn is_awaiting_first_answer(&self) -> bool {
        self.awaiting_list_answer || self.menu.is_awaiting_first_answer()
    }

    fn set_cursor_pos(&mut self, pos: (u16, u16)) {
        self.menu.set_cursor_pos(pos);
    }

    fn selection_at(
        &self,
        column: u16,
        menu_row: u16,
        terminal_columns: u16,
        available_lines: u16,
    ) -> Option<usize> {
        self.menu
            .selection_at(column, menu_row, terminal_columns, available_lines)
    }

    fn set_selection(&mut self, index: usize) {
        self.menu.set_selection(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reedline::{
        ColumnarMenu, CompletionResult, ListMenu, MenuBuilder, MenuEvent, UndoBehavior,
    };
    use rstest::rstest;

    #[derive(Default)]
    struct Recorder {
        seen: Vec<(String, usize)>,
    }

    impl Completer for Recorder {
        fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
            self.seen.push((line.to_string(), pos));
            CompletionResult::fresh(vec![Suggestion {
                value: "alpha".into(),
                ..Suggestion::default()
            }])
        }
    }

    struct Empty;
    impl Completer for Empty {
        fn complete(&mut self, _line: &str, _pos: usize) -> CompletionResult {
            CompletionResult::fresh(Vec::<Suggestion>::new())
        }
    }

    /// Answers from the handed line, like a live-mode `NuMenuCompleter`.
    #[derive(Default)]
    struct PrefixSpans {
        calls: usize,
    }
    impl Completer for PrefixSpans {
        fn complete(&mut self, line: &str, _pos: usize) -> CompletionResult {
            self.calls += 1;
            let start = line.rfind(' ').map(|i| i + 1).unwrap_or(0);
            CompletionResult::fresh(vec![
                Suggestion {
                    value: "rol".into(),
                    span: reedline::Span::new(start, line.len()),
                    ..Suggestion::default()
                },
                Suggestion {
                    value: "ror".into(),
                    span: reedline::Span::new(start, line.len()),
                    ..Suggestion::default()
                },
            ])
        }
    }

    fn active_menu() -> (Editor, SourcedMenu<ColumnarMenu>) {
        let editor = Editor::default();
        let mut menu = SourcedMenu::abandoning(ColumnarMenu::default());
        menu.menu_event(MenuEvent::Activate(false));
        assert!(menu.is_active());
        (editor, menu)
    }

    #[rstest]
    #[case::diff(InputMode::Diff)]
    #[case::cursor_prefix(InputMode::CursorPrefix)]
    #[case::full_buffer(InputMode::FullBuffer)]
    fn a_list_waits_for_its_first_answer_on_each_activation(#[case] input_mode: InputMode) {
        let mut editor = Editor::default();
        let mut completer = Recorder::default();
        let mut menu = SourcedMenu::new(
            ListMenu::default().with_input_mode(input_mode),
            MenuLine::default(),
            SourceMode::List {
                input_mode,
                suggested_span: true,
            },
        );

        menu.menu_event(MenuEvent::Activate(false));
        assert!(!menu.can_partially_complete(true, &mut editor, &mut completer));
        assert!(
            completer.seen.is_empty(),
            "the opening probe must not fetch"
        );
        assert!(menu.is_active());
        assert!(
            !menu.is_visible(),
            "hide the marker until the source answers"
        );
        menu.update_values(&mut editor, &mut completer);
        assert_eq!(completer.seen.len(), 1);
        assert!(menu.is_visible());

        menu.menu_event(MenuEvent::Edit(false));
        menu.update_values(&mut editor, &mut Empty);
        assert!(
            menu.is_visible(),
            "a later empty result keeps the list open"
        );
        menu.update_values(&mut editor, &mut completer);
        assert!(!menu.get_values().is_empty());

        menu.menu_event(MenuEvent::Deactivate);
        menu.menu_event(MenuEvent::Activate(false));
        assert!(
            menu.get_values().is_empty(),
            "hide rows from the last activation"
        );
        assert!(!menu.can_partially_complete(true, &mut editor, &mut Empty));
        assert!(menu.is_active());
        assert!(!menu.is_visible());
        menu.update_values(&mut editor, &mut Empty);
        assert!(!menu.is_active(), "a first empty answer closes the list");
        assert!(!menu.is_visible());
    }

    #[test]
    fn a_diff_menu_leaves_the_line_it_is_on() {
        let mut editor = Editor::default();
        editor.edit_buffer(
            |buffer| {
                buffer.set_buffer("ls ".into());
                buffer.set_insertion_point(3);
            },
            UndoBehavior::CreateUndoPoint,
        );

        let line = MenuLine::default();
        let mut menu = SourcedMenu::new(
            ColumnarMenu::default().with_input_mode(reedline::InputMode::Diff),
            line.clone(),
            SourceMode::Diff,
        );

        let mut recorder = Recorder::default();
        menu.update_values(&mut editor, &mut recorder);

        assert_eq!(line.read().as_deref(), Some("ls "));
        assert_eq!(recorder.seen.as_slice(), [(String::new(), 3)]);
    }

    #[test]
    fn empty_fresh_abandons_but_pending_does_not() {
        let (mut editor, mut menu) = active_menu();
        menu.update_values(&mut editor, &mut Empty);
        assert!(!menu.is_active(), "empty fresh must abandon");

        // Edit must not relaunch an abandoned menu, even with later values.
        menu.menu_event(MenuEvent::Edit(false));
        menu.update_values(&mut editor, &mut Empty);
        assert!(!menu.is_active());
        menu.menu_event(MenuEvent::Edit(false));
        menu.update_values(&mut editor, &mut Recorder::default());
        assert!(!menu.is_active(), "only Activate may reopen");

        // Provisional emptiness is still computing, not cancel.
        struct Pending;
        impl Completer for Pending {
            fn complete(&mut self, _line: &str, _pos: usize) -> CompletionResult {
                CompletionResult::Pending
            }
        }
        let (mut editor, mut menu) = active_menu();
        menu.update_values(&mut editor, &mut Pending);
        assert!(menu.is_active());
        assert!(menu.get_values().is_empty());

        // Non-empty keeps it open.
        let (mut editor, mut menu) = active_menu();
        menu.update_values(&mut editor, &mut Recorder::default());
        assert!(menu.is_active());
        assert_eq!(menu.get_values().len(), 1);
    }

    /// Typing past all matches empties a menu that already showed values; it must stay
    /// open and refill, since only a first empty answer is a cancelled picker (#19096).
    #[test]
    fn empty_after_values_stays_open_until_reactivated() {
        let (mut editor, mut menu) = active_menu();
        menu.update_values(&mut editor, &mut Recorder::default());
        menu.menu_event(MenuEvent::Edit(false));
        menu.update_values(&mut editor, &mut Empty);
        assert!(menu.is_active(), "no match after values must stay open");

        menu.menu_event(MenuEvent::Edit(false));
        menu.update_values(&mut editor, &mut Recorder::default());
        assert!(menu.is_active());
        assert_eq!(menu.get_values().len(), 1, "matching again must refill");

        // A new activation starts over: its first empty answer abandons again.
        menu.menu_event(MenuEvent::Deactivate);
        menu.menu_event(MenuEvent::Activate(false));
        menu.update_values(&mut editor, &mut Empty);
        assert!(!menu.is_active());
    }

    /// A partial splice must refresh a `Diff` source against the spliced line (#19053).
    /// `Diff` is the one mode where the recorded line is the source's only view of the
    /// full buffer, so the wrapper re-evaluation is required here -- and only here.
    #[test]
    fn diff_partial_completion_refreshes_the_source_against_the_spliced_line() {
        /// Answers from the recorded line, like `NuMenuCompleter` in `Diff` mode.
        struct RecordedLineSpans {
            line: MenuLine,
            calls: usize,
        }
        impl Completer for RecordedLineSpans {
            fn complete(&mut self, _line: &str, _pos: usize) -> CompletionResult {
                self.calls += 1;
                let buffer = self.line.read().unwrap_or_default();
                let start = buffer.rfind(' ').map(|i| i + 1).unwrap_or(0);
                CompletionResult::fresh(vec![
                    Suggestion {
                        value: "rol".into(),
                        span: reedline::Span::new(start, buffer.len()),
                        ..Suggestion::default()
                    },
                    Suggestion {
                        value: "ror".into(),
                        span: reedline::Span::new(start, buffer.len()),
                        ..Suggestion::default()
                    },
                ])
            }
        }

        let mut editor = Editor::default();
        editor.edit_buffer(
            |buffer| {
                buffer.set_buffer("bits r".into());
                buffer.set_insertion_point(6);
            },
            UndoBehavior::CreateUndoPoint,
        );

        let line = MenuLine::default();
        let mut menu = SourcedMenu::new(
            ColumnarMenu::default().with_input_mode(reedline::InputMode::Diff),
            line.clone(),
            SourceMode::Diff,
        );
        menu.menu_event(MenuEvent::Activate(false));
        let mut completer = RecordedLineSpans {
            line: line.clone(),
            calls: 0,
        };

        menu.update_values(&mut editor, &mut completer);
        assert_eq!(line.read().as_deref(), Some("bits r"));
        completer.calls = 0;

        assert!(
            menu.can_partially_complete(false, &mut editor, &mut completer),
            "the common prefix `o` should splice"
        );
        assert_eq!(editor.get_buffer(), "bits ro");
        assert_eq!(
            line.read().as_deref(),
            Some("bits ro"),
            "the source must see the spliced line, not the pre-splice one"
        );
        assert!(
            menu.get_values()
                .iter()
                .all(|s| s.span == reedline::Span::new(5, 7)),
            "values refreshed after the splice must span the spliced buffer, got {:?}",
            menu.get_values()
                .iter()
                .map(|s| (s.value.clone(), s.span))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            completer.calls, 3,
            "a Diff splice refreshes pre, post, and wrapper post"
        );
    }

    /// Outside `Diff` reedline hands the live buffer, so the inner post-splice
    /// evaluation already answers against it: no wrapper re-evaluation.
    #[test]
    fn cursor_prefix_partial_completion_reuses_the_handed_line() {
        let mut editor = Editor::default();
        editor.edit_buffer(
            |buffer| {
                buffer.set_buffer("bits r".into());
                buffer.set_insertion_point(6);
            },
            UndoBehavior::CreateUndoPoint,
        );

        let line = MenuLine::default();
        let mut menu = SourcedMenu::new(
            ColumnarMenu::default().with_input_mode(reedline::InputMode::CursorPrefix),
            line.clone(),
            SourceMode::Live,
        );
        menu.menu_event(MenuEvent::Activate(false));
        let mut completer = PrefixSpans { calls: 0 };

        menu.update_values(&mut editor, &mut completer);
        completer.calls = 0;

        assert!(
            menu.can_partially_complete(false, &mut editor, &mut completer),
            "the common prefix `o` should splice"
        );
        assert_eq!(editor.get_buffer(), "bits ro");
        assert_eq!(
            line.read().as_deref(),
            Some("bits ro"),
            "the recorded line stays fresh even without a re-evaluation"
        );
        assert!(
            menu.get_values()
                .iter()
                .all(|s| s.span == reedline::Span::new(5, 7)),
            "values must span the spliced buffer, got {:?}",
            menu.get_values()
                .iter()
                .map(|s| (s.value.clone(), s.span))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            completer.calls, 2,
            "a non-Diff splice pays only for the inner pre/post evaluations"
        );
    }

    /// Sourceless menus carry no line and already saw the spliced buffer, so a
    /// splicing Tab must not pay for a third engine evaluation.
    #[test]
    fn abandoning_partial_completion_skips_the_third_evaluation() {
        let mut editor = Editor::default();
        editor.edit_buffer(
            |buffer| {
                buffer.set_buffer("bits r".into());
                buffer.set_insertion_point(6);
            },
            UndoBehavior::CreateUndoPoint,
        );

        let mut menu = SourcedMenu::abandoning(ColumnarMenu::default());
        menu.menu_event(MenuEvent::Activate(false));
        let mut completer = PrefixSpans { calls: 0 };

        menu.update_values(&mut editor, &mut completer);
        completer.calls = 0;

        assert!(
            menu.can_partially_complete(false, &mut editor, &mut completer),
            "the common prefix `o` should splice"
        );
        assert_eq!(editor.get_buffer(), "bits ro");
        assert_eq!(
            completer.calls, 2,
            "abandoning menus must not add a wrapper evaluation"
        );
    }

    #[test]
    fn opening_probe_does_not_abandon_before_the_first_fetch() {
        // A probe told the values are current fetches nothing, so right after
        // Activate the values are empty only because the source has not run yet.
        // That probe must not close the menu; the first real fetch decides.
        let (mut editor, mut menu) = active_menu();
        menu.can_partially_complete(true, &mut editor, &mut Empty);
        assert!(
            menu.is_active(),
            "a just-opened menu with no values yet must survive the opening probe"
        );

        // The subsequent fetch is what may abandon it.
        menu.update_values(&mut editor, &mut Empty);
        assert!(!menu.is_active());
    }

    /// With quick completions off, the opening probe is the first fetch. A cancelled
    /// picker answers empty there, and the menu must close before it is painted, or
    /// the prompt keeps its indicator until the next keypress.
    #[test]
    fn opening_probe_that_fetches_empty_abandons() {
        let (mut editor, mut menu) = active_menu();
        menu.can_partially_complete(false, &mut editor, &mut Empty);
        assert!(!menu.is_active());
    }
}
