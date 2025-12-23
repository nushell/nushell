//! UI drawing functions and application loop for the regex explorer.

use crate::explore_regex::app::{App, InputFocus};
use crate::explore_regex::colors::styles;
use crate::explore_regex::quick_ref::QuickRefEntry;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};
use std::io::{self, Stdout};
use tui_textarea::{CursorMove, Input};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// ─── Key Action Handling ─────────────────────────────────────────────────────

/// Actions that can be triggered by keyboard input.
enum KeyAction {
    Quit,
    ToggleQuickRef,
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
    TextInput(Input),
    None,
}

/// Determine the action for a key event based on current app state.
fn determine_action(app: &App, key: &event::KeyEvent) -> KeyAction {
    // Global shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
        return KeyAction::Quit;
    }

    if key.code == KeyCode::F(1) {
        return KeyAction::ToggleQuickRef;
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

    // Default: pass to text input
    KeyAction::TextInput(Input::from(Event::Key(*key)))
}

/// Execute a key action, modifying app state.
fn execute_action(app: &mut App, action: KeyAction) -> bool {
    match action {
        KeyAction::Quit => return true,
        KeyAction::ToggleQuickRef => app.toggle_quick_ref(),
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
        KeyAction::TextInput(input) => handle_text_input(app, input),
        KeyAction::None => {}
    }
    false
}

fn handle_sample_page_navigation(app: &mut App, page_down: bool) {
    let page = app.sample_view_height.max(1);
    let (row, col) = app.sample_textarea.cursor();
    let max_row = app.sample_textarea.lines().len().saturating_sub(1) as u16;

    let target_row = if page_down {
        (row as u16).saturating_add(page).min(max_row)
    } else {
        (row as u16).saturating_sub(page)
    };

    app.sample_textarea
        .move_cursor(CursorMove::Jump(target_row, col as u16));
}

fn handle_text_input(app: &mut App, input: Input) {
    match app.input_focus {
        InputFocus::Regex => {
            app.regex_textarea.input(input);
            app.compile_regex();
        }
        InputFocus::Sample => {
            let old_text = app.get_sample_text();
            app.sample_textarea.input(input);
            if app.get_sample_text() != old_text {
                app.update_match_count();
            }
        }
        InputFocus::QuickRef => {}
    }
}

// ─── Main Loop ───────────────────────────────────────────────────────────────

pub fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| draw_ui(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        let action = determine_action(app, &key);
        if execute_action(app, action) {
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

    // Input block
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

    app.regex_textarea.set_block(block);
    app.regex_textarea.set_cursor_style(if focused {
        styles::cursor_active()
    } else {
        styles::cursor_hidden()
    });

    f.render_widget(&app.regex_textarea, input_area);
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

    // Handle scrolling
    if focused {
        update_sample_scroll(app, content);
    }

    let text = app.get_highlighted_text();
    let paragraph = Paragraph::new(text)
        .scroll((app.sample_scroll_v, app.sample_scroll_h))
        .block(block);

    f.render_widget(paragraph, content_area);

    // Draw cursor
    if focused {
        draw_sample_cursor(f, app, content);
    }
}

fn update_sample_scroll(app: &mut App, content: Rect) {
    let (cursor_row, cursor_col) = app.sample_textarea.cursor();
    let line = &app.sample_textarea.lines()[cursor_row];
    let cursor_display_col = line
        .graphemes(true)
        .take(cursor_col)
        .map(|g| g.width())
        .sum::<usize>() as u16;
    let cursor_row_u16 = cursor_row as u16;

    // Vertical scrolling
    if cursor_row_u16 < app.sample_scroll_v {
        app.sample_scroll_v = cursor_row_u16;
    } else if cursor_row_u16 >= app.sample_scroll_v + content.height {
        app.sample_scroll_v = cursor_row_u16 - content.height + 1;
    }

    // Horizontal scrolling
    if cursor_display_col < app.sample_scroll_h {
        app.sample_scroll_h = cursor_display_col;
    } else if cursor_display_col >= app.sample_scroll_h + content.width {
        app.sample_scroll_h = cursor_display_col - content.width + 1;
    }
}

fn draw_sample_cursor(f: &mut ratatui::Frame, app: &App, content: Rect) {
    let buf = f.buffer_mut();
    let (cursor_row, cursor_col) = app.sample_textarea.cursor();
    let line = &app.sample_textarea.lines()[cursor_row];
    let prefix_width = line
        .graphemes(true)
        .take(cursor_col)
        .map(|g| g.width())
        .sum::<usize>() as u16;

    let cursor_x = content.x + prefix_width - app.sample_scroll_h;
    let cursor_y = content.y + (cursor_row as u16) - app.sample_scroll_v;
    let grapheme_count = line.graphemes(true).count();
    let is_eol = cursor_col == grapheme_count;

    let grapheme_width = if is_eol {
        1
    } else {
        line.graphemes(true)
            .nth(cursor_col)
            .map(|g| g.width())
            .unwrap_or(1)
    };

    for i in 0..grapheme_width {
        if let Some(cell) = buf.cell_mut((cursor_x + i as u16, cursor_y)) {
            if is_eol {
                cell.set_symbol(" ");
            }
            cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
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
