//! UI drawing functions and application loop for the regex explorer.

use crate::explore_regex::app::{App, InputFocus};
use crate::explore_regex::colors::{BG_DARK, FG_PRIMARY, styles};
use crate::explore_regex::quick_ref::QuickRefEntry;
use edtui::{
    EditorEventHandler, EditorMode, EditorTheme, EditorView,
    actions::{DeleteChar, DeleteCharForward, Paste},
    events::{KeyEventRegister, KeyInput},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Widget,
    },
};
use std::io::{self, Stdout};
use unicode_width::UnicodeWidthStr;

// ─── Key Action Handling ─────────────────────────────────────────────────────

/// Actions that can be triggered by keyboard input.
enum KeyAction {
    Quit,
    ToggleQuickRef,
    ShowHelp,
    CloseHelp,
    SwitchFocus,
    FocusRegex,
    QuickRefUp,
    QuickRefDown,
    QuickRefPageUp,
    QuickRefPageDown,
    QuickRefLeft,
    QuickRefRight,
    QuickRefHome,
    QuickRefInsert,
    SamplePageUp,
    SamplePageDown,
    PassToEditor(event::KeyEvent),
    None,
}

/// Determine the appropriate action for a key event based on current application state.
///
/// This function implements the key event routing logic:
/// - Help modal captures all keys to close
/// - Global shortcuts (Ctrl+Q, F1, F2) work everywhere
/// - Quick reference panel has its own navigation keys when focused
/// - Regex input blocks newline insertion (single-line field)
/// - All other keys are passed to the editor for text input
fn determine_action(app: &App, key: &event::KeyEvent) -> KeyAction {
    // If help modal is shown, any key closes it
    if app.show_help {
        return KeyAction::CloseHelp;
    }

    // Global shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
        return KeyAction::Quit;
    }

    if key.code == KeyCode::F(1) {
        return KeyAction::ToggleQuickRef;
    }

    if key.code == KeyCode::F(2) {
        return KeyAction::ShowHelp;
    }

    // Quick reference panel navigation
    if app.show_quick_ref && app.input_focus == InputFocus::QuickRef {
        return match key.code {
            KeyCode::Up | KeyCode::Char('k') => KeyAction::QuickRefUp,
            KeyCode::Down | KeyCode::Char('j') => KeyAction::QuickRefDown,
            KeyCode::PageUp => KeyAction::QuickRefPageUp,
            KeyCode::PageDown => KeyAction::QuickRefPageDown,
            KeyCode::Left | KeyCode::Char('h') => KeyAction::QuickRefLeft,
            KeyCode::Right | KeyCode::Char('l') => KeyAction::QuickRefRight,
            KeyCode::Home => KeyAction::QuickRefHome,
            KeyCode::Enter => KeyAction::QuickRefInsert,
            KeyCode::Esc | KeyCode::Tab | KeyCode::BackTab => KeyAction::FocusRegex,
            _ => KeyAction::None,
        };
    }

    // Focus switching
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        return KeyAction::SwitchFocus;
    }

    if key.code == KeyCode::Esc {
        return KeyAction::FocusRegex;
    }

    // Sample pane page navigation
    if app.input_focus == InputFocus::Sample {
        match key.code {
            KeyCode::PageUp => return KeyAction::SamplePageUp,
            KeyCode::PageDown => return KeyAction::SamplePageDown,
            _ => {}
        }
    }

    // Prevent newlines in regex input (single-line field)
    // Block Enter, Ctrl+J, and Ctrl+M which all insert newlines in edtui
    if app.input_focus == InputFocus::Regex {
        match key.code {
            KeyCode::Enter => return KeyAction::None,
            KeyCode::Char('j') | KeyCode::Char('m')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                return KeyAction::None;
            }
            _ => {}
        }
    }

    // Default: pass to editor
    KeyAction::PassToEditor(*key)
}

/// Execute a key action, modifying application state as needed.
///
/// Returns `true` if the application should quit, `false` otherwise.
fn execute_action(
    app: &mut App,
    action: KeyAction,
    event_handler: &mut EditorEventHandler,
) -> bool {
    match action {
        KeyAction::Quit => return true,
        KeyAction::ToggleQuickRef => app.toggle_quick_ref(),
        KeyAction::ShowHelp => app.toggle_help(),
        KeyAction::CloseHelp => app.show_help = false,
        KeyAction::SwitchFocus => {
            app.input_focus = match app.input_focus {
                InputFocus::Regex => InputFocus::Sample,
                InputFocus::Sample | InputFocus::QuickRef => InputFocus::Regex,
            };
        }
        KeyAction::FocusRegex => {
            if app.show_quick_ref && app.input_focus == InputFocus::QuickRef {
                app.close_quick_ref();
            } else {
                app.input_focus = InputFocus::Regex;
            }
        }
        KeyAction::QuickRefUp => app.quick_ref_up(),
        KeyAction::QuickRefDown => app.quick_ref_down(),
        KeyAction::QuickRefPageUp => app.quick_ref_page_up(),
        KeyAction::QuickRefPageDown => app.quick_ref_page_down(),
        KeyAction::QuickRefLeft => app.quick_ref_scroll_left(),
        KeyAction::QuickRefRight => app.quick_ref_scroll_right(),
        KeyAction::QuickRefHome => app.quick_ref_scroll_home(),
        KeyAction::QuickRefInsert => app.insert_selected_quick_ref(),
        KeyAction::SamplePageUp | KeyAction::SamplePageDown => {
            handle_sample_page_navigation(app, matches!(action, KeyAction::SamplePageDown));
        }
        KeyAction::PassToEditor(key) => handle_editor_input(app, key, event_handler),
        KeyAction::None => {}
    }
    false
}

/// Handle page up/down navigation in the sample text pane.
///
/// Moves the cursor up or down by one page (visible height), keeping the column position.
fn handle_sample_page_navigation(app: &mut App, page_down: bool) {
    let page = app.sample_view_height.max(1);
    let row = app.sample_text.cursor.row;
    let col = app.sample_text.cursor.col;
    let max_row = app.sample_text.lines.len().saturating_sub(1);

    let target_row = if page_down {
        row.saturating_add(page as usize).min(max_row)
    } else {
        row.saturating_sub(page as usize)
    };

    app.sample_text.cursor.row = target_row;
    app.sample_text.cursor.col = col;
}

/// Normalize AltGr key events by stripping Ctrl+Alt modifiers from non-alphabetic character keys.
///
/// On many international keyboards (e.g., Swiss German, German), AltGr is used to type
/// characters like `\`, `{`, `}`, `[`, `]`, `~`, etc. These key events are reported as
/// `Ctrl+Alt+Char` by crossterm/Windows. However, edtui interprets `Ctrl+Alt`
/// combinations as control sequences rather than character input.
///
/// To distinguish between AltGr character input and intentional keybindings:
/// - ASCII letters (a-z, A-Z) with Ctrl+Alt or Alt are treated as keybindings
///   (e.g., Alt+f for word-forward, Ctrl+Alt+b for move-to-head)
/// - Non-alphabetic characters with Ctrl+Alt or Alt are treated as AltGr input
///   (e.g., AltGr+[ to type `[`, AltGr+{ to type `{`)
///
/// This heuristic works because:
/// 1. All edtui Alt/Ctrl+Alt keybindings use letters (f, b, h, d, n, p, v, etc.)
/// 2. AltGr typically produces symbols/punctuation, not letters
/// 3. edtui only inserts characters in Insert mode if modifiers are NONE or SHIFT
///
/// Without this normalization, Swiss-German keyboard users cannot type regex-critical
/// characters like `[`, `]`, `{`, `}`, `\`, `|`, `@` in the regex input field.
fn normalize_altgr_key(key: &event::KeyEvent) -> event::KeyEvent {
    if let KeyCode::Char(c) = key.code {
        // AltGr is typically reported as Ctrl+Alt on Windows/some terminals
        // Some terminals may report it as just Alt
        let has_altgr_modifiers = key
            .modifiers
            .contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
            || key.modifiers == KeyModifiers::ALT;

        if has_altgr_modifiers {
            // Only treat as AltGr character input if it's NOT an ASCII letter.
            // ASCII letters with Alt/Ctrl+Alt are likely intentional keybindings
            // (e.g., Alt+f for word-forward, Ctrl+Alt+b for move-to-head).
            // Symbols/punctuation with Ctrl+Alt are likely AltGr character input
            // (e.g., AltGr+ü for [ on Swiss German keyboard).
            if !c.is_ascii_alphabetic() {
                // Strip Ctrl+Alt, keep only Shift if present
                let new_modifiers = key.modifiers & KeyModifiers::SHIFT;
                return event::KeyEvent::new_with_kind_and_state(
                    key.code,
                    new_modifiers,
                    key.kind,
                    key.state,
                );
            }
        }
    }

    // Return the key unchanged for:
    // - Non-Char keys (Backspace, Delete, arrows, etc.)
    // - ASCII letters with Alt/Ctrl+Alt (keybindings)
    // - Characters without Alt modifiers (regular typing)
    *key
}

/// Pass a key event to the editor and handle side effects.
///
/// For regex input: recompiles the regex if the text changed.
/// For sample text: updates match count if the text changed.
fn handle_editor_input(
    app: &mut App,
    key: event::KeyEvent,
    event_handler: &mut EditorEventHandler,
) {
    // Normalize AltGr keys so international keyboards (Swiss-German, etc.) work correctly
    let normalized_key = normalize_altgr_key(&key);

    match app.input_focus {
        InputFocus::Regex => {
            let old_value = app.regex_input.lines.to_string();
            event_handler.on_key_event(normalized_key, &mut app.regex_input);
            if app.regex_input.lines.to_string() != old_value {
                app.compile_regex();
            }
        }
        InputFocus::Sample => {
            let old_text = app.get_sample_text();
            event_handler.on_key_event(normalized_key, &mut app.sample_text);
            if app.get_sample_text() != old_text {
                app.update_match_count();
            }
        }
        InputFocus::QuickRef => {}
    }
}

// ─── Main Loop ───────────────────────────────────────────────────────────────

/// Main event loop for the regex explorer.
///
/// Sets up custom keybindings to fix edtui bugs and improve UX, then enters the main
/// draw/event loop. Returns when the user quits (Ctrl+Q) or an error occurs.
pub fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    use ratatui::crossterm::event::KeyCode as CTKeyCode;

    // Create event handler for edtui in Emacs mode (modeless editing)
    let mut event_handler = EditorEventHandler::emacs_mode();

    // Fix and customize edtui's Emacs mode keybindings for better UX:
    //
    // edtui has several bugs and unconventional defaults in Emacs mode:
    // 1. Backspace is mapped TWICE - the second mapping (forward delete) overwrites the first,
    //    causing backspace to delete forward instead of backward
    // 2. Delete key (forward delete) has no mapping at all
    // 3. Ctrl+V is mapped to page-down (traditional Emacs), but modern users expect paste
    //
    // We override these to provide intuitive behavior:

    use edtui::actions::Action;

    let keybindings: [(CTKeyCode, Action); 2] = [
        // Fix: Backspace should delete backward (edtui bug causes it to delete forward)
        (CTKeyCode::Backspace, DeleteChar(1).into()),
        // Add: Delete key should delete forward (missing in edtui's Emacs mode)
        (CTKeyCode::Delete, DeleteCharForward(1).into()),
    ];

    for (key_code, action) in keybindings {
        event_handler.key_handler.insert(
            KeyEventRegister::new(vec![KeyInput::new(key_code)], EditorMode::Insert),
            action,
        );
    }

    // Override Ctrl+V to paste (edtui maps it to page-down by default)
    // Modern users expect Ctrl+V for paste; Emacs users can still use Ctrl+Y
    event_handler.key_handler.insert(
        KeyEventRegister::new(vec![KeyInput::ctrl('v')], EditorMode::Insert),
        Paste,
    );

    loop {
        terminal.draw(|f| draw_ui(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        let action = determine_action(app, &key);
        if execute_action(app, action, &mut event_handler) {
            return Ok(());
        }
    }
}

// ─── UI Drawing ──────────────────────────────────────────────────────────────

fn draw_ui(f: &mut ratatui::Frame, app: &mut App) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_unfocused())
        .title(Line::from(vec![Span::styled(
            " Regex Explorer ",
            styles::focused(),
        )]))
        .title_alignment(Alignment::Left);

    let inner_area = outer_block.inner(f.area());
    f.render_widget(outer_block, f.area());

    if app.show_quick_ref {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(40), Constraint::Length(40)])
            .split(inner_area);

        draw_main_content(f, app, chunks[0]);
        draw_quick_ref_panel(f, app, chunks[1]);
    } else {
        draw_main_content(f, app, inner_area);
    }

    if app.show_help {
        draw_help_modal_overlay(f, app, f.area());
    }
}

fn draw_main_content(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Regex label
            Constraint::Length(3), // Regex input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Sample label
            Constraint::Min(6),    // Sample
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Help
        ])
        .horizontal_margin(2)
        .split(area);

    draw_regex_section(f, app, chunks[1], chunks[2]);
    draw_sample_section(f, app, chunks[4], chunks[5]);
    draw_help(f, app, chunks[7]);
}

// ─── Section Drawing Helpers ─────────────────────────────────────────────────

fn draw_regex_section(f: &mut ratatui::Frame, app: &mut App, label_area: Rect, input_area: Rect) {
    let focused = app.input_focus == InputFocus::Regex;

    // Label with status
    let status = match (&app.regex_error, &app.compiled_regex) {
        (Some(_), _) => Some(("invalid", styles::status_error())),
        (None, Some(_)) => Some(("valid", styles::status_success())),
        _ => None,
    };

    let label = build_label(
        "Regex Pattern",
        focused,
        status.map(|(t, s)| (t.to_string(), s)),
    );
    f.render_widget(Paragraph::new(label), label_area);

    // Border style
    let border_style = if focused {
        if app.regex_error.is_some() {
            styles::border_error()
        } else {
            styles::border_focused()
        }
    } else {
        styles::border_unfocused()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .padding(Padding::horizontal(1));

    let content = block.inner(input_area);

    // Render using EditorView with theme (hide cursor, we'll use terminal cursor)
    let theme = EditorTheme::default()
        .block(block)
        .base(Style::default().fg(FG_PRIMARY).bg(BG_DARK))
        .hide_cursor() // Hide EditorView's block cursor
        .hide_status_line(); // Hide the "Insert" mode indicator
    EditorView::new(&mut app.regex_input)
        .theme(theme)
        .render(input_area, f.buffer_mut());

    // Set terminal cursor position if focused
    if focused {
        let cursor_col = app.regex_input.cursor.col;
        let cursor_row = app.regex_input.cursor.row;
        f.set_cursor_position((content.x + cursor_col as u16, content.y + cursor_row as u16));
    }
}

fn draw_sample_section(
    f: &mut ratatui::Frame,
    app: &mut App,
    label_area: Rect,
    content_area: Rect,
) {
    let focused = app.input_focus == InputFocus::Sample;

    // Label with match count
    let status: Option<(String, Style)> = if app.match_count > 0 {
        let text = if app.match_count == 1 {
            "1 match".to_string()
        } else {
            format!("{} matches", app.match_count)
        };
        Some((text, styles::separator()))
    } else if app.compiled_regex.is_some() {
        Some(("no matches".to_string(), styles::status_warning()))
    } else {
        None
    };

    let label = build_label("Test String", focused, status);
    f.render_widget(Paragraph::new(label), label_area);

    // Sample block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            styles::border_focused()
        } else {
            styles::border_unfocused()
        })
        .padding(Padding::horizontal(1));

    let content = block.inner(content_area);
    app.sample_view_height = content.height;

    // Render the block border
    f.render_widget(block, content_area);

    // Render the highlighted text with regex matches
    let highlighted_text = app.get_highlighted_text();
    f.render_widget(Paragraph::new(highlighted_text), content);

    // Set terminal cursor position if this section is focused
    if focused {
        let cursor_row = app.sample_text.cursor.row;
        let cursor_col = app.sample_text.cursor.col;

        // Only set if cursor is within visible area
        if cursor_row < content.height as usize {
            f.set_cursor_position((content.x + cursor_col as u16, content.y + cursor_row as u16));
        }
    }
}

// ─── Label Building Helpers ──────────────────────────────────────────────────

/// Build a label line with optional status badge.
fn build_label(
    title: &str,
    focused: bool,
    status: Option<(impl Into<String>, Style)>,
) -> Line<'static> {
    let mut spans = if focused {
        vec![
            Span::styled("> ", styles::focus_indicator()),
            Span::styled(title.to_string(), styles::focused()),
        ]
    } else {
        vec![
            Span::styled("  ", styles::unfocused()),
            Span::styled(title.to_string(), styles::unfocused()),
        ]
    };

    if let Some((text, style)) = status {
        spans.push(Span::styled("  [", styles::status_bracket()));
        spans.push(Span::styled(text.into(), style));
        spans.push(Span::styled("]", styles::status_bracket()));
    }

    Line::from(spans)
}

// ─── Help Bar ────────────────────────────────────────────────────────────────

fn draw_help(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let sep = Span::styled("  •  ", styles::separator());

    let mut spans = vec![
        help_key("Tab"),
        help_desc(" Switch Focus"),
        sep.clone(),
        help_key("Esc"),
        help_desc(" Focus Regex"),
        sep.clone(),
        help_key("F1"),
        help_desc(if app.show_quick_ref {
            " Hide Quick Ref"
        } else {
            " Quick Ref"
        }),
        sep.clone(),
        help_key("F2"),
        help_desc(" Help"),
        sep.clone(),
        help_key("Ctrl+Q"),
        help_desc(" Exit"),
    ];

    if app.show_quick_ref && app.input_focus == InputFocus::QuickRef {
        spans.push(sep);
        spans.push(help_key("↑↓"));
        spans.push(help_desc(" Navigate"));
        spans.push(Span::styled("  ", styles::separator()));
        spans.push(help_key("←→"));
        spans.push(help_desc(" Scroll"));
        spans.push(Span::styled("  ", styles::separator()));
        spans.push(help_key("Enter"));
        spans.push(help_desc(" Insert"));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn help_key(text: &str) -> Span<'static> {
    Span::styled(text.to_string(), styles::focused())
}

fn help_desc(text: &str) -> Span<'static> {
    Span::styled(text.to_string(), styles::separator())
}

// ─── Quick Reference Panel ───────────────────────────────────────────────────

fn draw_quick_ref_panel(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let focused = app.input_focus == InputFocus::QuickRef;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            styles::border_focused()
        } else {
            styles::border_unfocused()
        })
        .title(Line::from(vec![Span::styled(
            " Quick Reference ",
            styles::focused(),
        )]))
        .title_alignment(Alignment::Center)
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    let visible_width = inner.width;
    app.quick_ref_view_height = visible_height;
    app.quick_ref_view_width = visible_width;

    // Adjust scroll to keep selected visible
    if app.quick_ref_selected < app.quick_ref_scroll {
        app.quick_ref_scroll = app.quick_ref_selected;
    } else if app.quick_ref_selected >= app.quick_ref_scroll + visible_height {
        app.quick_ref_scroll = app.quick_ref_selected - visible_height + 1;
    }

    // Build content lines
    let lines: Vec<Line> = app
        .quick_ref_entries
        .iter()
        .enumerate()
        .skip(app.quick_ref_scroll)
        .take(visible_height)
        .map(|(idx, entry)| build_quick_ref_line(entry, idx, app.quick_ref_selected, focused))
        .collect();

    let paragraph = Paragraph::new(lines).scroll((0, app.quick_ref_scroll_h));
    f.render_widget(paragraph, inner);

    // Scrollbar
    if app.quick_ref_entries.len() > visible_height {
        draw_scrollbar(f, area, app.quick_ref_entries.len(), app.quick_ref_scroll);
    }
}

fn build_quick_ref_line(
    entry: &QuickRefEntry,
    idx: usize,
    selected: usize,
    focused: bool,
) -> Line<'static> {
    const SYNTAX_WIDTH: usize = 14;

    match entry {
        QuickRefEntry::Category(name) => Line::from(vec![Span::styled(
            format!("─ {} ─────────────────────────────────────", name),
            styles::category_header(),
        )]),
        QuickRefEntry::Item(item) => {
            let is_selected = idx == selected && focused;
            let syntax = format!("{:<width$}", item.syntax, width = SYNTAX_WIDTH);

            if is_selected {
                Line::from(vec![
                    Span::styled(syntax, styles::selected_bold()),
                    Span::styled(" ", styles::selected()),
                    Span::styled(item.description.to_string(), styles::selected()),
                    // Extra padding for smooth horizontal scrolling
                    Span::styled("          ", styles::selected()),
                ])
            } else {
                Line::from(vec![
                    Span::styled(syntax, styles::focused()),
                    Span::styled(" ", styles::unfocused()),
                    Span::styled(item.description.to_string(), styles::unfocused()),
                ])
            }
        }
    }
}

fn draw_scrollbar(f: &mut ratatui::Frame, area: Rect, total: usize, position: usize) {
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    let mut state = ScrollbarState::new(total).position(position);

    let scrollbar_area = Rect {
        x: area.x + area.width - 2,
        y: area.y + 1,
        width: 1,
        height: area.height.saturating_sub(2),
    };

    f.render_stateful_widget(scrollbar, scrollbar_area, &mut state);
}

// ─── Help Modal Overlay ───────────────────────────────────────────────────────

fn draw_help_modal_overlay(f: &mut ratatui::Frame, _app: &App, area: Rect) {
    // Define help content
    let help_lines = vec![
        "Global Shortcuts",
        "  Ctrl+Q       Exit",
        "  F2           Toggle Help",
        "  F1           Toggle Quick Ref",
        "  Tab          Switch Focus",
        "  Esc          Focus Regex",
        "",
        "Quick Reference Panel",
        "  ↑↓ / jk      Navigate",
        "  ←→ / hl      Scroll",
        "  Enter        Insert",
        "  PgUp/PgDn    Page Scroll",
        "  Home         Scroll to Start",
        "",
        "Regex Pattern Pane (single line)",
        "  ←→                    Move cursor",
        "  Ctrl+F/B              Forward/Back char",
        "  Ctrl+A/E              Line head/end",
        "  Alt+F/B               Forward/Back word",
        "  Backspace/Ctrl+H      Delete char before",
        "  Delete/Ctrl+D         Delete char after",
        "  Ctrl+K                Delete to line end",
        "  Alt+U                 Delete to line head",
        "  Alt+Backspace         Delete word before",
        "  Alt+D                 Delete word after",
        "  Ctrl+U                Undo",
        "  Ctrl+R                Redo",
        "  Ctrl+V / Ctrl+Y       Paste from clipboard",
        "",
        "Test String Pane (multi-line) (same as above plus:)",
        "  Ctrl+N/P              Next/Previous line",
        "  Enter/Ctrl+J          Insert newline",
        "",
        "Press any key to close",
    ];

    // Calculate required dimensions
    let content_height = help_lines.len() as u16;
    let content_width = help_lines
        .iter()
        .map(|line| line.width() as u16)
        .max()
        .unwrap_or(30);

    // Add padding and borders
    let modal_width = (content_width + 6).min(area.width - 4);
    let modal_height = (content_height + 4).min(area.height - 4);

    let modal_x = (area.width - modal_width) / 2;
    let modal_y = (area.height - modal_height) / 2;
    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

    // Modal background style using existing color scheme
    let modal_bg = Style::default().bg(BG_DARK).fg(FG_PRIMARY);

    // Fill the entire modal area with solid background color by directly writing to buffer
    // This ensures complete opacity - Clear widget alone doesn't fill with a color
    let buf = f.buffer_mut();
    for y in modal_area.y..modal_area.y + modal_area.height {
        for x in modal_area.x..modal_area.x + modal_area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_style(modal_bg);
            }
        }
    }

    // Modal block
    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(styles::border_focused().bg(BG_DARK))
        .style(modal_bg)
        .title(Line::from(vec![Span::styled(
            " Keybindings Help ",
            styles::focused().bg(BG_DARK),
        )]))
        .title_alignment(Alignment::Center)
        .padding(Padding::horizontal(2));

    let inner_area = modal_block.inner(modal_area);
    f.render_widget(modal_block, modal_area);

    // Convert to styled lines using existing color scheme
    let help_text: Vec<Line> = help_lines
        .into_iter()
        .map(|line| {
            if line.is_empty() {
                Line::from(Span::styled(" ", modal_bg))
            } else if line.starts_with("  ") {
                // Key-value line
                let parts: Vec<&str> = line.splitn(2, "  ").collect();
                if parts.len() == 2 {
                    Line::from(vec![
                        Span::styled(parts[0].trim_end(), styles::focused().bg(BG_DARK)),
                        Span::styled(" ", modal_bg),
                        Span::styled(parts[1], styles::modal_desc().bg(BG_DARK)),
                    ])
                } else {
                    Line::from(vec![Span::styled(line, styles::modal_desc().bg(BG_DARK))])
                }
            } else {
                // Header
                Line::from(vec![Span::styled(
                    line,
                    styles::category_header().bg(BG_DARK),
                )])
            }
        })
        .collect();

    let paragraph = Paragraph::new(help_text)
        .style(modal_bg)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .scroll((0, 0));

    f.render_widget(paragraph, inner_area);
}
