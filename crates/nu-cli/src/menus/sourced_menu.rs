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
        *self.0.lock() = Some(line.to_string());
    }

    /// Last recorded line.
    pub(crate) fn read(&self) -> Option<String> {
        self.0.lock().clone()
    }
}

/// How a menu source sees the editor line: `Diff` hands only the fragment typed
/// since the menu opened (parse the recorded line), every other mode hands the
/// live buffer. Classified once via [`SourceMode::of`]; both the completer and
/// its wrapper store the result, so they cannot disagree (#19053).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceMode {
    Diff,
    Live,
}

impl SourceMode {
    pub(crate) fn of(mode: &InputMode) -> Self {
        match mode {
            InputMode::Diff => Self::Diff,
            _ => Self::Live,
        }
    }

    /// What the source parses, with a floored/clamped cursor.
    pub(crate) fn buffer(
        self,
        line: &MenuLine,
        handed: &str,
        pos: usize,
        replacing_start: usize,
    ) -> (String, usize) {
        let buffer = match self {
            Self::Diff => line
                .read()
                .unwrap_or_else(|| " ".repeat(replacing_start) + handed),
            Self::Live => handed.to_owned(),
        };
        let cursor = buffer.floor_char_boundary(pos.min(buffer.len()));
        (buffer, cursor)
    }
}

/// Menu wrapper carrying the editor line to its source, and abandoning the menu on a
/// fresh empty answer so a cancelled picker (fzf Esc / `input list` Esc) cannot leave
/// an empty menu that relaunches on type.
pub struct SourcedMenu<M> {
    menu: M,
    line: Option<MenuLine>,
    mode: SourceMode,
}

impl<M> SourcedMenu<M> {
    pub fn new(menu: M, line: MenuLine, mode: SourceMode) -> Self {
        Self {
            menu,
            line: Some(line),
            mode,
        }
    }

    /// Engine-completer menus have no source line to carry, only abandon-on-empty.
    pub fn abandoning(menu: M) -> Self {
        Self {
            menu,
            line: None,
            mode: SourceMode::Live,
        }
    }

    fn record(&self, editor: &Editor) {
        if let Some(line) = &self.line {
            line.record(editor.get_buffer());
        }
    }

    /// Close on a fresh empty answer; provisional/awaiting emptiness is still computing.
    fn abandon_if_empty(&mut self)
    where
        M: Menu,
    {
        if self.menu.is_active()
            && self.menu.get_values().is_empty()
            && !self.menu.results_are_provisional()
            && !self.menu.is_awaiting_first_answer()
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
        // No abandon check here!!! reedline probes partial completion when a menu
        // opens, even before the source has ever run, so the values are legitimately
        // empty.
        let spliced = self
            .menu
            .can_partially_complete(values_updated, editor, completer);
        if spliced {
            // The splice ran past this wrapper, so a `Diff` source would answer
            // with pre-splice spans without a refresh (#19053); anything else
            // already saw the spliced buffer through the handed line.
            match (self.mode, &self.line) {
                (SourceMode::Diff, Some(_)) => self.update_values(editor, completer),
                _ => {
                    self.record(editor);
                    self.abandon_if_empty();
                }
            }
        }
        spliced
    }

    fn update_values(&mut self, editor: &mut Editor, completer: &mut dyn Completer) {
        self.record(editor);
        self.menu.update_values(editor, completer);
        self.abandon_if_empty();
    }

    fn reset_position(&mut self) {
        self.menu.reset_position();
    }

    fn reload(&mut self, updated: bool, editor: &mut Editor, completer: &mut dyn Completer) {
        self.record(editor);
        self.menu.reload(updated, editor, completer);
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
        self.menu.get_values()
    }

    fn results_are_provisional(&self) -> bool {
        self.menu.results_are_provisional()
    }

    fn is_awaiting_first_answer(&self) -> bool {
        self.menu.is_awaiting_first_answer()
    }

    fn set_cursor_pos(&mut self, pos: (u16, u16)) {
        self.menu.set_cursor_pos(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reedline::{ColumnarMenu, CompletionResult, MenuBuilder, MenuEvent, UndoBehavior};

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
        // Reedline probes partial completion right after Activate, while the
        // source has not run yet and the values are still empty. That probe
        // must not close the menu; the first real fetch decides.
        let (mut editor, mut menu) = active_menu();
        menu.can_partially_complete(false, &mut editor, &mut Empty);
        assert!(
            menu.is_active(),
            "a just-opened menu with no values yet must survive the opening probe"
        );

        // The subsequent fetch is what may abandon it.
        menu.update_values(&mut editor, &mut Empty);
        assert!(!menu.is_active());
    }
}
