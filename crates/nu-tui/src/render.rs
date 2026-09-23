//! Draw a [`Session`] onto a ratatui frame: the dialog chrome, every widget
//! in its area, then the overlays (tab bar, split handles, open dropdown).
use crate::session::Session;
use crate::theme::Theme;
use crate::widget::{WidgetKind, WidgetState};
use crate::widgets::label::Slot;
use crate::widgets::split::SplitDir;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(frame: &mut Frame, session: &mut Session) {
    let theme = session.theme.clone();
    if session.dialog.is_some() {
        frame.render_widget(Block::default().style(theme.backdrop()), frame.area());
        render_dialog_frame(frame, session, &theme);
    }
    let content = session.dialog_content_area(frame.area());
    session.layout(content);

    let ids: Vec<String> = session.app.iter().map(|w| w.id.clone()).collect();
    for id in ids {
        let Some(area) = session.areas.get(&id).copied() else {
            continue;
        };
        if area.width == 0 || area.height == 0 {
            continue;
        }
        let (Some(widget), Some(state)) = (session.widget(&id), session.state(&id)) else {
            continue;
        };
        let focused = session.is_focused(&id);
        widget
            .kind
            .render(&id, state, frame, area, session, focused);
    }

    render_tabs(frame, session, &theme);
    render_splitter_handles(frame, session, &theme);
    render_menu_dropdown(frame, session);
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
            WidgetKind::Label(l) if l.slot == Slot::Title => Some(l.text(&w.id, session)),
            _ => None,
        })
        .unwrap_or_else(|| "tui".into());
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
    for handle in &session.handles {
        if handle.area.width == 0 || handle.area.height == 0 {
            continue;
        }
        let focused = session.is_focused(&handle.id);
        let ch = match handle.direction {
            SplitDir::Horizontal => "│",
            SplitDir::Vertical => "─",
        };
        let fill = match handle.direction {
            SplitDir::Horizontal => vec![ch; handle.area.height as usize].join("\n"),
            SplitDir::Vertical => ch.repeat(handle.area.width as usize),
        };
        frame.render_widget(
            Paragraph::new(fill).style(theme.border(focused)),
            handle.area,
        );
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
        frame.render_widget(
            Paragraph::new(Span::styled(format!(" {title} "), theme.tab(active))),
            *area,
        );
    }
}

fn render_menu_dropdown(frame: &mut Frame, session: &Session) {
    for w in session.app.iter() {
        let WidgetKind::Menu(menu) = &w.kind else {
            continue;
        };
        let Some(state) = session.state(&w.id).and_then(WidgetState::as_menu) else {
            continue;
        };
        let Some(bar) = session.areas.get(&w.id) else {
            continue;
        };
        if let Some(rect) = menu.dropdown_rect(state, *bar) {
            menu.render_dropdown(state, rect, frame, session);
        }
    }
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
            match buffer.cell((x, y)) {
                Some(cell) if !cell.symbol().is_empty() => line.push_str(cell.symbol()),
                _ => line.push(' '),
            }
        }
        output.push_str(line.trim_end());
        output.push('\n');
    }
    Ok(output)
}
