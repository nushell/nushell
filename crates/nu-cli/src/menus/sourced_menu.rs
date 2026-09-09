use reedline::{Completer, Editor, Menu, MenuEvent, Painter, Suggestion};
use std::sync::{Arc, Mutex};

/// Shared command line for menu source.
#[derive(Clone, Default)]
pub struct MenuLine(Arc<Mutex<Option<String>>>);

impl MenuLine {
    /// Record current editor line.
    pub(crate) fn record(&self, line: &str) {
        if let Ok(mut recorded) = self.0.lock() {
            *recorded = Some(line.to_string());
        }
    }

    /// Last recorded line.
    pub(crate) fn read(&self) -> Option<String> {
        self.0.lock().ok()?.clone()
    }
}

/// Menu wrapper carrying the editor line to its source, and abandoning the menu on a
/// fresh empty answer so a cancelled picker (fzf Esc / `input list` Esc) cannot leave
/// an empty menu that relaunches on type.
pub struct SourcedMenu<M> {
    menu: M,
    line: Option<MenuLine>,
}

impl<M> SourcedMenu<M> {
    pub fn new(menu: M, line: MenuLine) -> Self {
        Self {
            menu,
            line: Some(line),
        }
    }

    /// Engine-completer menus have no source line to carry, only abandon-on-empty.
    pub fn abandoning(menu: M) -> Self {
        Self { menu, line: None }
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
        let done = self
            .menu
            .can_partially_complete(values_updated, editor, completer);
        self.abandon_if_empty();
        done
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
    use reedline::{
        ColumnarMenu, CompletionResult, InputMode, MenuBuilder, MenuEvent, UndoBehavior,
    };

    struct Recorder(Arc<Mutex<Vec<(String, usize)>>>);

    impl Completer for Recorder {
        fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
            if let Ok(mut seen) = self.0.lock() {
                seen.push((line.to_string(), pos));
            }
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
            ColumnarMenu::default().with_input_mode(InputMode::Diff),
            line.clone(),
        );

        let seen = Arc::new(Mutex::new(Vec::new()));
        menu.update_values(&mut editor, &mut Recorder(Arc::clone(&seen)));

        assert_eq!(line.read().as_deref(), Some("ls "));
        assert_eq!(
            seen.lock().expect("what the menu handed over").as_slice(),
            [(String::new(), 3)]
        );
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
        menu.update_values(&mut editor, &mut Recorder(Arc::new(Mutex::new(Vec::new()))));
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
        menu.update_values(&mut editor, &mut Recorder(Arc::new(Mutex::new(Vec::new()))));
        assert!(menu.is_active());
        assert_eq!(menu.get_values().len(), 1);
    }
}
