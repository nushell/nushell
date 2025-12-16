//! UI drawing functions and application loop for the regex explorer.

use crate::explore_regex::app::{App, InputFocus};
use crate::explore_regex::colors;
use crate::explore_regex::quick_ref::QuickRefEntry;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
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

        // Handle Ctrl+Q to quit
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(());
        }

        // Handle F1 or '?' (without modifiers, when not in text input) to toggle quick reference panel
        if key.code == KeyCode::F(1) {
            app.toggle_quick_ref();
            continue;
        }

        // Handle quick reference panel navigation when it's open and focused
        if app.show_quick_ref && matches!(app.input_focus, InputFocus::QuickRef) {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.quick_ref_up();
                    continue;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.quick_ref_down();
                    continue;
                }
                KeyCode::PageUp => {
                    app.quick_ref_page_up();
                    continue;
                }
                KeyCode::PageDown => {
                    app.quick_ref_page_down();
                    continue;
                }
                KeyCode::Enter => {
                    app.insert_selected_quick_ref();
                    // Keep the panel open so user can add more patterns
                    continue;
                }
                KeyCode::Esc => {
                    app.show_quick_ref = false;
                    app.input_focus = InputFocus::Regex;
                    continue;
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    // Tab out of quick ref panel to regex/sample
                    app.input_focus = InputFocus::Regex;
                    continue;
                }
                _ => continue,
            }
        }

        // Handle Tab to switch focus (when quick ref panel is not focused)
        if key.code == KeyCode::Tab || key.code == KeyCode::BackTab {
            app.input_focus = match app.input_focus {
                InputFocus::Regex => InputFocus::Sample,
                InputFocus::Sample => InputFocus::Regex,
                InputFocus::QuickRef => InputFocus::Regex,
            };
            continue;
        }

        // Escape will focus the Regex field back again (or close quick ref if open)
        if key.code == KeyCode::Esc {
            if app.show_quick_ref {
                app.show_quick_ref = false;
            }
            app.input_focus = InputFocus::Regex;
            continue;
        }

        // Intercept PageUp/PageDown in Sample pane to move by one page height
        if matches!(app.input_focus, InputFocus::Sample)
            && matches!(key.code, KeyCode::PageUp | KeyCode::PageDown)
        {
            let page = app.sample_view_height.max(1);
            let (row, col) = app.sample_textarea.cursor();
            let max_row = app.sample_textarea.lines().len().saturating_sub(1) as u16;

            let target_row = match key.code {
                KeyCode::PageUp => (row as u16).saturating_sub(page),
                KeyCode::PageDown => (row as u16).saturating_add(page).min(max_row),
                _ => row as u16,
            };

            app.sample_textarea
                .move_cursor(CursorMove::Jump(target_row, col as u16));
            continue;
        }

        // Convert crossterm event to tui-textarea input
        let input = Input::from(Event::Key(key));

        // Handle input based on current mode
        match app.input_focus {
            InputFocus::Regex => {
                app.regex_textarea.input(input);
                app.compile_regex(); // TODO: Do this in a worker thread.
            }
            InputFocus::Sample => {
                // Track if text content actually changed (not just cursor movement)
                let old_text = app.get_sample_text();
                app.sample_textarea.input(input);
                let new_text = app.get_sample_text();

                // Only update match count if the text content changed
                if old_text != new_text {
                    app.update_match_count();
                }
            }
            InputFocus::QuickRef => {
                // Already handled above
            }
        }
    }
}

fn draw_ui(f: &mut ratatui::Frame, app: &mut App) {
    // Main layout with outer border
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(colors::FG_MUTED))
        .title(Line::from(vec![Span::styled(
            " Regex Explorer ",
            Style::new().fg(colors::FG_PRIMARY).bold(),
        )]))
        .title_alignment(Alignment::Left);

    let inner_area = outer_block.inner(f.area());
    f.render_widget(outer_block, f.area());

    // If quick reference panel is shown, split horizontally
    if app.show_quick_ref {
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(40),    // Main content (at least 40 columns)
                Constraint::Length(40), // Quick reference panel (fixed 40 columns)
            ])
            .split(inner_area);

        draw_main_content(f, app, horizontal_chunks[0]);
        draw_quick_ref_panel(f, app, horizontal_chunks[1]);
    } else {
        draw_main_content(f, app, inner_area);
    }
}

fn draw_main_content(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Regex label + status
            Constraint::Length(3), // Regex input (with border)
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Sample label + match count
            Constraint::Min(6),    // Sample
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Help
        ])
        .horizontal_margin(2)
        .split(area);

    draw_body(f, app, (chunks[1], chunks[2], chunks[4], chunks[5]));
    draw_help(f, app, chunks[7]);
}

fn draw_help(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let key_style = Style::new().fg(colors::FG_PRIMARY).bold();
    let desc_style = Style::new().fg(colors::FG_MUTED);
    let separator = Span::styled("  •  ", Style::new().fg(colors::FG_MUTED));

    let mut spans = vec![
        Span::styled("Tab", key_style),
        Span::styled(" Switch Focus", desc_style),
        separator.clone(),
        Span::styled("Esc", key_style),
        Span::styled(" Focus Regex", desc_style),
        separator.clone(),
        Span::styled("F1", key_style),
        if app.show_quick_ref {
            Span::styled(" Hide Help", desc_style)
        } else {
            Span::styled(" Quick Ref", desc_style)
        },
        separator.clone(),
        Span::styled("Ctrl+Q", key_style),
        Span::styled(" Exit", desc_style),
    ];

    // Add quick ref navigation hints when panel is open
    if app.show_quick_ref && matches!(app.input_focus, InputFocus::QuickRef) {
        spans.push(separator);
        spans.push(Span::styled("↑↓", key_style));
        spans.push(Span::styled(" Navigate", desc_style));
        spans.push(Span::styled("  ", desc_style));
        spans.push(Span::styled("Enter", key_style));
        spans.push(Span::styled(" Insert", desc_style));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_quick_ref_panel(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let focused = matches!(app.input_focus, InputFocus::QuickRef);

    let border_color = if focused {
        colors::ACCENT
    } else {
        colors::FG_MUTED
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border_color))
        .title(Line::from(vec![Span::styled(
            " Quick Reference ",
            Style::new().fg(colors::FG_PRIMARY).bold(),
        )]))
        .title_alignment(Alignment::Center)
        .padding(Padding::horizontal(1));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner_area.height as usize;
    app.quick_ref_view_height = visible_height;

    // Adjust scroll to keep selected item visible
    if app.quick_ref_selected < app.quick_ref_scroll {
        app.quick_ref_scroll = app.quick_ref_selected;
    } else if app.quick_ref_selected >= app.quick_ref_scroll + visible_height {
        app.quick_ref_scroll = app.quick_ref_selected - visible_height + 1;
    }

    // Build lines for the quick reference content
    let mut lines: Vec<Line> = Vec::new();

    for (idx, entry) in app.quick_ref_entries.iter().enumerate() {
        // Skip entries before scroll position
        if idx < app.quick_ref_scroll {
            continue;
        }
        // Stop if we've filled the visible area
        if lines.len() >= visible_height {
            break;
        }

        let is_selected = idx == app.quick_ref_selected && focused;

        match entry {
            QuickRefEntry::Category(name) => {
                // Category header with special styling
                let header_style = Style::new()
                    .fg(colors::ACCENT)
                    .bold()
                    .add_modifier(Modifier::UNDERLINED);
                lines.push(Line::from(vec![Span::styled(
                    format!("─ {} ", name),
                    header_style,
                )]));
            }
            QuickRefEntry::Item(item) => {
                // Calculate available width for description
                let syntax_width = 12; // Fixed width for syntax column
                let available_width = inner_area.width.saturating_sub(syntax_width + 3) as usize;

                // Truncate description if needed
                let description = if item.description.len() > available_width {
                    format!(
                        "{}…",
                        &item.description[..available_width.saturating_sub(1)]
                    )
                } else {
                    item.description.to_string()
                };

                let (syntax_style, desc_style) = if is_selected {
                    (
                        Style::new().fg(colors::BG_DARK).bg(colors::ACCENT).bold(),
                        Style::new().fg(colors::BG_DARK).bg(colors::ACCENT),
                    )
                } else {
                    (
                        Style::new().fg(colors::FG_PRIMARY).bold(),
                        Style::new().fg(colors::FG_MUTED),
                    )
                };

                // Format syntax with fixed width padding
                let syntax_formatted =
                    format!("{:<width$}", item.syntax, width = syntax_width as usize);

                // Build the line with selection highlight extending full width
                if is_selected {
                    let remaining_width = inner_area
                        .width
                        .saturating_sub(syntax_width + description.len() as u16 + 1)
                        as usize;
                    let padding = " ".repeat(remaining_width);
                    lines.push(Line::from(vec![
                        Span::styled(syntax_formatted, syntax_style),
                        Span::styled(" ", desc_style),
                        Span::styled(description, desc_style),
                        Span::styled(padding, desc_style),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(syntax_formatted, syntax_style),
                        Span::styled(" ", desc_style),
                        Span::styled(description, desc_style),
                    ]));
                }
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner_area);

    // Render scrollbar if content is scrollable
    let total_entries = app.quick_ref_entries.len();
    if total_entries > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state = ScrollbarState::new(total_entries).position(app.quick_ref_scroll);

        // Render scrollbar in the area to the right of content
        let scrollbar_area = Rect {
            x: area.x + area.width - 2,
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };

        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

fn draw_body(f: &mut ratatui::Frame, app: &mut App, areas: (Rect, Rect, Rect, Rect)) {
    let focused_regex = matches!(app.input_focus, InputFocus::Regex);
    let focused_sample = matches!(app.input_focus, InputFocus::Sample);

    // Build regex label with status indicator
    let regex_status: Vec<Span> = if app.regex_error.is_some() {
        vec![
            Span::styled("  [", Style::new().fg(colors::FG_MUTED)),
            Span::styled("invalid", Style::new().fg(colors::ERROR)),
            Span::styled("]", Style::new().fg(colors::FG_MUTED)),
        ]
    } else if app.compiled_regex.is_some() {
        vec![
            Span::styled("  [", Style::new().fg(colors::FG_MUTED)),
            Span::styled("valid", Style::new().fg(colors::SUCCESS)),
            Span::styled("]", Style::new().fg(colors::FG_MUTED)),
        ]
    } else {
        vec![]
    };

    let mut regex_label_line: Vec<Span> = if focused_regex {
        vec![
            Span::styled("> ", Style::new().fg(colors::FG_PRIMARY)),
            Span::styled("Regex Pattern", Style::new().fg(colors::FG_PRIMARY).bold()),
        ]
    } else {
        vec![
            Span::styled("  ", Style::new().fg(colors::FG_MUTED)),
            Span::styled("Regex Pattern", Style::new().fg(colors::FG_MUTED)),
        ]
    };
    regex_label_line.extend(regex_status);
    f.render_widget(Paragraph::new(Line::from(regex_label_line)), areas.0);

    // Regex input block
    let regex_border_color = if focused_regex {
        if app.regex_error.is_some() {
            colors::ERROR
        } else {
            colors::ACCENT
        }
    } else {
        colors::FG_MUTED
    };

    let regex_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(regex_border_color))
        .padding(Padding::horizontal(1));

    app.regex_textarea.set_block(regex_block);
    app.regex_textarea.set_cursor_style(if focused_regex {
        Style::new().bg(colors::ACCENT).fg(colors::BG_DARK)
    } else {
        Style::new().hidden()
    });
    f.render_widget(&app.regex_textarea, areas.1);

    // Build sample label with match count
    let match_count_span: Vec<Span> = if app.match_count > 0 {
        let match_text = if app.match_count == 1 {
            "1 match".to_string()
        } else {
            format!("{} matches", app.match_count)
        };
        vec![
            Span::styled("  [", Style::new().fg(colors::FG_MUTED)),
            Span::styled(match_text, Style::new().fg(colors::FG_MUTED)),
            Span::styled("]", Style::new().fg(colors::FG_MUTED)),
        ]
    } else if app.compiled_regex.is_some() {
        vec![
            Span::styled("  [", Style::new().fg(colors::FG_MUTED)),
            Span::styled("no matches", Style::new().fg(colors::WARNING)),
            Span::styled("]", Style::new().fg(colors::FG_MUTED)),
        ]
    } else {
        vec![]
    };

    let mut sample_label_line: Vec<Span> = if focused_sample {
        vec![
            Span::styled("> ", Style::new().fg(colors::FG_PRIMARY)),
            Span::styled("Test String", Style::new().fg(colors::FG_PRIMARY).bold()),
        ]
    } else {
        vec![
            Span::styled("  ", Style::new().fg(colors::FG_MUTED)),
            Span::styled("Test String", Style::new().fg(colors::FG_MUTED)),
        ]
    };
    sample_label_line.extend(match_count_span);
    f.render_widget(Paragraph::new(Line::from(sample_label_line)), areas.2);

    // Sample block
    let sample_border_color = if focused_sample {
        colors::ACCENT
    } else {
        colors::FG_MUTED
    };

    let sample_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(sample_border_color))
        .padding(Padding::horizontal(1));

    let content_area = sample_block.inner(areas.3);
    let visible_rows = content_area.height;
    let visible_cols = content_area.width;
    app.sample_view_height = visible_rows;

    if focused_sample {
        let (cursor_row, cursor_col) = app.sample_textarea.cursor();
        let line = &app.sample_textarea.lines()[cursor_row];
        let cursor_display_col = line[0..cursor_col].width() as u16;
        let cursor_row_u16 = cursor_row as u16;

        // Vertical scrolling
        if cursor_row_u16 < app.sample_scroll_v {
            app.sample_scroll_v = cursor_row_u16;
        } else if cursor_row_u16 >= app.sample_scroll_v + visible_rows {
            app.sample_scroll_v = cursor_row_u16 - visible_rows + 1;
        }

        // Horizontal scrolling
        if cursor_display_col < app.sample_scroll_h {
            app.sample_scroll_h = cursor_display_col;
        } else if cursor_display_col >= app.sample_scroll_h + visible_cols {
            app.sample_scroll_h = cursor_display_col - visible_cols + 1;
        }
    }

    let highlighted_text = app.get_highlighted_text();
    let text_paragraph = Paragraph::new(highlighted_text)
        .scroll((app.sample_scroll_v, app.sample_scroll_h))
        .block(sample_block);

    f.render_widget(text_paragraph, areas.3);

    if focused_sample {
        let buf = f.buffer_mut();
        let (cursor_row, cursor_col) = app.sample_textarea.cursor();
        let line = &app.sample_textarea.lines()[cursor_row];
        let prefix_width = line[0..cursor_col].width() as u16;
        let relative_col = prefix_width - app.sample_scroll_h;
        let relative_row = (cursor_row as u16) - app.sample_scroll_v;
        let cursor_x = content_area.x + relative_col;
        let cursor_y = content_area.y + relative_row;
        let is_eol = cursor_col == line.len();

        let grapheme_width = if is_eol {
            1
        } else {
            line[cursor_col..]
                .graphemes(true)
                .next()
                .map(|g| g.width())
                .unwrap_or(1)
        };

        for i in 0..grapheme_width {
            let x = cursor_x + i as u16;
            let y = cursor_y;

            if let Some(cell) = buf.cell_mut((x, y)) {
                if is_eol {
                    cell.set_symbol(" ");
                }
                cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
            }
        }
    }
}
