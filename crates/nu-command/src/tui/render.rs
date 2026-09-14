//! Draw a [`Session`](super::session::Session) onto a ratatui frame.
use super::session::{Session, format_event_value, formatted_key};
use super::theme::Theme;
use super::widget::WidgetKind;
use ansi_str::get_blocks;
use nu_protocol::Value;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap};

pub fn render(frame: &mut Frame, session: &mut Session) {
    let theme = session.theme.clone();
    if session.dialog.is_some() {
        frame.render_widget(Block::default().style(theme.backdrop()), frame.area());
        render_dialog_frame(frame, session, &theme);
    }
    let content = session.dialog_content_area(frame.area());
    session.layout(content);

    let ids: Vec<String> = session.app.widgets.iter().map(|w| w.id.clone()).collect();
    for id in ids {
        let Some(area) = session.areas.get(&id).copied() else {
            continue;
        };
        if area.width == 0 || area.height == 0 {
            continue;
        }
        render_widget(frame, session, &id, area, &theme);
    }

    render_tabs(frame, session, &theme);
    render_splitter_handles(frame, session, &theme);
}

fn render_dialog_frame(frame: &mut Frame, session: &Session, theme: &Theme) {
    let Some(dialog) = &session.dialog else {
        return;
    };
    let title = session
        .app
        .widgets
        .iter()
        .find_map(|w| match &w.kind {
            WidgetKind::Title { text } => Some(text.as_str()),
            _ => None,
        })
        .unwrap_or("tui");
    frame.render_widget(Block::default().style(theme.surface()), dialog.rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .style(theme.surface())
        .border_style(theme.border(true))
        .title(format!(" {title} "))
        .title(
            Line::from(Span::styled(
                " x ",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Right),
        );
    frame.render_widget(block, dialog.rect);
    let grip = Rect {
        x: dialog
            .rect
            .x
            .saturating_add(dialog.rect.width.saturating_sub(1)),
        y: dialog
            .rect
            .y
            .saturating_add(dialog.rect.height.saturating_sub(1)),
        width: 1,
        height: 1,
    };
    frame.render_widget(Paragraph::new("┘").style(theme.highlight()), grip);
}

fn render_splitter_handles(frame: &mut Frame, session: &Session, theme: &Theme) {
    for handle in &session.splitter_handles {
        if handle.area.width == 0 || handle.area.height == 0 {
            continue;
        }
        let focused = session.is_focused(&handle.id);
        let ch = match handle.direction {
            super::widget::SplitDir::Horizontal => "│",
            super::widget::SplitDir::Vertical => "─",
        };
        frame.render_widget(Paragraph::new(ch).style(theme.border(focused)), handle.area);
    }
}

fn render_tabs(frame: &mut Frame, session: &Session, theme: &Theme) {
    if session.tab_areas.is_empty() {
        return;
    }
    let pages = session.pages();
    for (i, area) in session.tab_areas.iter().enumerate() {
        let title = pages.get(i).map(|p| p.title.as_str()).unwrap_or("page");
        let active = i == session.page;
        let style = if active {
            Style::default()
                .fg(theme.tab_active)
                .bg(theme.surface)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.tab_inactive).bg(theme.surface)
        };
        frame.render_widget(
            Paragraph::new(Span::styled(format!(" {title} "), style)),
            *area,
        );
    }
}

fn render_widget(frame: &mut Frame, session: &Session, id: &str, area: Rect, theme: &Theme) {
    let Some(kind) = session.app.widget_kind(id) else {
        return;
    };
    let focused = session.is_focused(id);
    match kind {
        WidgetKind::Title { text } => {
            let para = Paragraph::new(format!(" {text} ")).style(theme.title());
            frame.render_widget(para, area);
        }
        WidgetKind::Menu { items } => render_menu(frame, session, id, items, area, theme, focused),
        WidgetKind::Label { text } => {
            frame.render_widget(Paragraph::new(text.as_str()).style(theme.text()), area);
        }
        WidgetKind::TextBox { placeholder, .. } => {
            render_textbox(frame, session, id, placeholder, area, theme, focused)
        }
        WidgetKind::Table { .. } => render_table(frame, session, id, area, theme, focused, "table"),
        WidgetKind::List { .. } => render_table(frame, session, id, area, theme, focused, "list"),
        WidgetKind::Log { .. } => render_log(frame, session, id, area, theme, focused),
        WidgetKind::Tree { .. } => render_tree(frame, session, id, area, theme, focused),
        WidgetKind::Body { title } | WidgetKind::Tab { title } => {
            let block = Block::default()
                .borders(Borders::ALL)
                .style(theme.surface())
                .border_style(theme.border(focused))
                .title(format!(" {title} "));
            frame.render_widget(block, area);
        }
        WidgetKind::Status { .. } => {
            let text = session.status_text();
            frame.render_widget(
                Paragraph::new(format!(" {text} ")).style(theme.status()),
                area,
            );
        }
        WidgetKind::Keybindings { .. } => {
            render_table(frame, session, id, area, theme, focused, "keybindings")
        }
        WidgetKind::Search { placeholder, .. } => {
            render_search(frame, session, placeholder, area, theme, focused)
        }
        WidgetKind::Splitter { direction, .. } => {
            let ch = match direction {
                super::widget::SplitDir::Horizontal => "│",
                super::widget::SplitDir::Vertical => "─",
            };
            let style = theme.border(focused);
            frame.render_widget(Paragraph::new(ch).style(style), area);
        }
        WidgetKind::Preview { .. } => render_preview(frame, session, id, area, theme),
        WidgetKind::Tabs => {}
    }
}

fn render_tree(
    frame: &mut Frame,
    session: &Session,
    id: &str,
    area: Rect,
    theme: &Theme,
    focused: bool,
) {
    let rows = session.tree_rows(id);
    let selected = session.selected.get(id).copied().unwrap_or(0);
    let scroll = session.scroll.get(id).copied().unwrap_or(0);
    let count = rows.len();
    let block = Block::default()
        .borders(Borders::ALL)
        .style(theme.surface())
        .border_style(theme.border(focused))
        .title(format!(" tree ({count}) "));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let height = inner.height as usize;
    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(i, row)| {
            let marker = if !row.expandable {
                "  "
            } else if row.expanded {
                "▾ "
            } else {
                "▸ "
            };
            let indent = "  ".repeat(row.depth);
            let mut style = session.tree_node_style(&row.value);
            if i == selected {
                style = style.patch(theme.selected());
            }
            Line::from(Span::styled(
                format!("{indent}{marker}{}", row.label),
                style,
            ))
        })
        .collect();
    frame.render_widget(Paragraph::new(Text::from(lines)).style(theme.text()), inner);
}

fn render_log(
    frame: &mut Frame,
    session: &Session,
    id: &str,
    area: Rect,
    theme: &Theme,
    focused: bool,
) {
    let rows = session.filtered_rows(id);
    let max_lines = match session.app.widget_kind(id) {
        Some(WidgetKind::Log { max_lines }) => *max_lines,
        _ => rows.len(),
    };
    let start = rows.len().saturating_sub(max_lines);
    let lines: Vec<Line> = rows
        .iter()
        .skip(start)
        .map(|v| ansi_string_to_line(&compact_line(v), theme.surface))
        .collect();
    let follow = *session.follow_tail.get(id).unwrap_or(&true);
    let title = if session.stream_live {
        " log (live) "
    } else if follow {
        " log "
    } else {
        " log (paused) "
    };
    let scroll = session.scroll.get(id).copied().unwrap_or(0) as u16;
    let block = Block::default()
        .borders(Borders::ALL)
        .style(theme.surface())
        .border_style(theme.border(focused))
        .title(title);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .style(theme.text())
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
            .block(block),
        area,
    );
}

fn compact_line(value: &Value) -> String {
    match value {
        Value::String { val, .. } => val.clone(),
        Value::Nothing { .. } => String::new(),
        other => other.to_expanded_string(" ", &nu_protocol::Config::default()),
    }
}

fn render_preview(frame: &mut Frame, session: &Session, id: &str, area: Rect, theme: &Theme) {
    let title = session
        .preview_title
        .get(id)
        .cloned()
        .unwrap_or_else(|| "preview".into());
    let text = session.preview_text.get(id).cloned().unwrap_or_default();
    let scroll = session.scroll.get(id).copied().unwrap_or(0) as u16;
    let block = Block::default()
        .borders(Borders::ALL)
        .style(theme.surface())
        .border_style(theme.border(false))
        .title(format!(" {title} "));
    let para = if text.is_empty() {
        Paragraph::new("(not a file)")
            .style(theme.muted())
            .block(block)
    } else {
        Paragraph::new(ansi_to_text(&text, theme.surface))
            .style(theme.text())
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
            .block(block)
    };
    frame.render_widget(para, area);
}

fn ansi_to_text(text: &str, fallback_bg: Color) -> Text<'static> {
    let lines: Vec<Line<'static>> = text
        .split('\n')
        .map(|line| ansi_string_to_line(line, fallback_bg))
        .collect();
    Text::from(lines)
}

fn ansi_string_to_line(ansi_text: &str, fallback_bg: Color) -> Line<'static> {
    let mut spans = Vec::new();
    for block in get_blocks(ansi_text) {
        let style = ansi_style_to_ratatui(block.style(), fallback_bg);
        spans.push(Span::styled(block.text().to_string(), style));
    }
    if spans.is_empty() {
        Line::from(Span::styled(
            String::new(),
            Style::default().bg(fallback_bg),
        ))
    } else {
        Line::from(spans)
    }
}

fn ansi_style_to_ratatui(style: &ansi_str::Style, fallback_bg: Color) -> Style {
    let mut out = Style::default().bg(fallback_bg);
    if let Some(clr) = style.foreground() {
        out.fg = ansi_color_to_ratatui(clr);
    }
    if let Some(clr) = style.background() {
        out.bg = ansi_color_to_ratatui(clr);
    }
    if style.is_bold() {
        out.add_modifier |= Modifier::BOLD;
    }
    if style.is_faint() {
        out.add_modifier |= Modifier::DIM;
    }
    if style.is_italic() {
        out.add_modifier |= Modifier::ITALIC;
    }
    if style.is_underline() {
        out.add_modifier |= Modifier::UNDERLINED;
    }
    out
}

fn ansi_color_to_ratatui(clr: ansi_str::Color) -> Option<Color> {
    use ansi_str::Color::*;
    Some(match clr {
        Black => Color::Black,
        BrightBlack => Color::DarkGray,
        Red => Color::Red,
        BrightRed => Color::LightRed,
        Green => Color::Green,
        BrightGreen => Color::LightGreen,
        Yellow => Color::Yellow,
        BrightYellow => Color::LightYellow,
        Blue => Color::Blue,
        BrightBlue => Color::LightBlue,
        Magenta => Color::Magenta,
        BrightMagenta => Color::LightMagenta,
        Cyan => Color::Cyan,
        BrightCyan => Color::LightCyan,
        White => Color::White,
        BrightWhite => Color::Gray,
        Purple => Color::Magenta,
        BrightPurple => Color::LightMagenta,
        Fixed(i) => Color::Indexed(i),
        Rgb(r, g, b) => Color::Rgb(r, g, b),
    })
}

fn render_menu(
    frame: &mut Frame,
    session: &Session,
    id: &str,
    items: &[String],
    area: Rect,
    theme: &Theme,
    focused: bool,
) {
    let selected = session.selected.get(id).copied().unwrap_or(0);
    let mut spans = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let style = if i == selected {
            theme.selected()
        } else {
            theme.text()
        };
        spans.push(Span::styled(format!(" {item} "), style));
    }
    let bg = if focused {
        theme.surface().fg(theme.border_focused)
    } else {
        theme.surface()
    };
    frame.render_widget(Paragraph::new(Line::from(spans)).style(bg), area);
}

fn render_textbox(
    frame: &mut Frame,
    session: &Session,
    id: &str,
    placeholder: &str,
    area: Rect,
    theme: &Theme,
    focused: bool,
) {
    let editable = matches!(
        session.app.widget_kind(id),
        Some(WidgetKind::TextBox { editable: true, .. })
    );
    let value = session.text_values.get(id).cloned().unwrap_or_default();
    let cursor = session
        .text_cursors
        .get(id)
        .copied()
        .unwrap_or(value.chars().count());
    let block = Block::default()
        .borders(Borders::ALL)
        .style(theme.surface())
        .border_style(theme.border(focused && editable))
        .title(" input ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let display = if value.is_empty() {
        Paragraph::new(placeholder).style(theme.muted())
    } else if focused && editable {
        Paragraph::new(with_cursor(&value, cursor, theme)).style(theme.text())
    } else {
        Paragraph::new(value).style(theme.text())
    };
    frame.render_widget(display, inner);
}

fn render_search(
    frame: &mut Frame,
    session: &Session,
    placeholder: &str,
    area: Rect,
    theme: &Theme,
    focused: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(theme.surface())
        .border_style(theme.border(focused))
        .title(" search ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let display = if session.search.is_empty() {
        Paragraph::new(placeholder).style(theme.muted())
    } else if focused {
        Paragraph::new(with_cursor(&session.search, session.search_cursor, theme))
            .style(theme.highlight())
    } else {
        Paragraph::new(session.search.as_str()).style(theme.highlight())
    };
    frame.render_widget(display, inner);
}

fn render_table(
    frame: &mut Frame,
    session: &Session,
    id: &str,
    area: Rect,
    theme: &Theme,
    focused: bool,
    title: &str,
) {
    let columns = session.table_columns(id);
    let computer = session.style_computer();
    let rows_data = display_rows(session, id, &columns, computer.as_ref());
    let selected = session.selected.get(id).copied().unwrap_or(0);
    let n = columns.len().max(1);
    let widths: Vec<ratatui::layout::Constraint> = columns
        .iter()
        .map(|_| ratatui::layout::Constraint::Percentage((100 / n) as u16))
        .collect();

    let header = Row::new(
        columns
            .iter()
            .map(|c| Cell::from(c.as_str()).style(theme.header())),
    )
    .height(1);

    let rows: Vec<Row> = rows_data
        .iter()
        .map(|row| {
            Row::new(
                row.iter()
                    .cloned()
                    .map(|(text, style)| Cell::from(text).style(style)),
            )
            .height(1)
        })
        .collect();

    let count = rows.len();
    let title = format!(" {title} ({count}) ");
    let table = Table::new(rows, widths)
        .header(header)
        .style(theme.table_body())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(theme.surface())
                .border_style(theme.border(focused))
                .title(title),
        )
        .row_highlight_style(theme.selected())
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    if count > 0 {
        state.select(Some(selected.min(count - 1)));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn display_rows(
    session: &Session,
    id: &str,
    columns: &[String],
    computer: Option<&nu_color_config::StyleComputer>,
) -> Vec<Vec<(String, Style)>> {
    let is_keys = matches!(
        session.app.widget_kind(id),
        Some(WidgetKind::Keybindings { .. })
    );
    session
        .filtered_rows(id)
        .into_iter()
        .map(|row| {
            if is_keys {
                keybinding_cells(&row)
                    .into_iter()
                    .map(|text| (text, session.theme.text()))
                    .collect()
            } else {
                columns
                    .iter()
                    .map(|col| session.styled_cell(&row, col, computer))
                    .collect()
            }
        })
        .collect()
}

fn keybinding_cells(row: &Value) -> Vec<String> {
    let Ok(record) = row.as_record() else {
        return vec![compact(row), String::new(), String::new(), String::new()];
    };
    let name = record.get("name").map(compact).unwrap_or_default();
    let key = formatted_key(record);
    let mode = record.get("mode").map(format_mode).unwrap_or_default();
    let event = record
        .get("event")
        .map(format_event_value)
        .unwrap_or_default();
    vec![name, key, mode, event]
}

fn format_mode(value: &Value) -> String {
    match value {
        Value::List { vals, .. } => vals
            .iter()
            .filter_map(|v| v.as_str().ok())
            .collect::<Vec<_>>()
            .join(","),
        Value::String { val, .. } => val.clone(),
        other => compact(other),
    }
}

fn compact(value: &Value) -> String {
    match value {
        Value::Nothing { .. } => String::new(),
        Value::String { val, .. } => val.clone(),
        other => other.to_expanded_string(", ", &nu_protocol::Config::default()),
    }
}

fn with_cursor(text: &str, cursor: usize, theme: &Theme) -> Line<'static> {
    let cursor = cursor.min(text.chars().count());
    let mut chars = text.chars();
    let before: String = chars.by_ref().take(cursor).collect();
    let after: String = chars.collect();
    Line::from(vec![
        Span::styled(before, theme.text()),
        Span::styled("▌", theme.text().add_modifier(Modifier::SLOW_BLINK)),
        Span::styled(after, theme.text()),
    ])
}

/// Render a session to a string using ratatui's test backend.
pub fn render_to_string(session: &mut Session, width: u16, height: u16) -> Result<String, String> {
    use ratatui::{Terminal, backend::TestBackend};

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    terminal
        .draw(|frame| render(frame, session))
        .map_err(|e| e.to_string())?;
    let buffer = terminal.backend().buffer();
    let mut output = String::new();
    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            if let Some(cell) = buffer.cell((x, y)) {
                let symbol = cell.symbol();
                if symbol.is_empty() {
                    line.push(' ');
                } else {
                    line.push_str(symbol);
                }
            } else {
                line.push(' ');
            }
        }
        output.push_str(line.trim_end());
        output.push('\n');
    }
    Ok(output)
}
