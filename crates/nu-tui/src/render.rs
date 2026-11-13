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
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

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
    render_pending_keys(frame, session, &theme);
}

/// The key panel: while a bind sequence is pending, the keys that continue
/// it and what they do, in columns above the status line.
fn render_pending_keys(frame: &mut Frame, session: &Session, theme: &Theme) {
    let keys = session.pending_keys();
    if keys.is_empty() {
        return;
    }
    let screen = frame.area();
    let entry_width = keys
        .iter()
        .map(|(key, label)| key.width() + 3 + label.width() + 2)
        .max()
        .unwrap_or(1);
    let columns =
        (screen.width.saturating_sub(2) as usize / entry_width.max(1)).clamp(1, keys.len());
    let rows = keys.len().div_ceil(columns);
    let height = (rows as u16).saturating_add(2).min(screen.height);
    let bottom = session.status_area().map_or(screen.bottom(), |s| s.y);
    let rect = Rect::new(
        screen.x,
        bottom.saturating_sub(height).max(screen.y),
        screen.width,
        height,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border(true))
        .style(theme.surface())
        .title(format!(" {} ", session.pending.join(" ")));
    let inner = block.inner(rect);
    frame.render_widget(Clear, rect);
    frame.render_widget(block, rect);
    // Filled down each column first, as in yazi.
    for (i, (key, label)) in keys.iter().enumerate() {
        let x = inner.x.saturating_add(((i / rows) * entry_width) as u16);
        let y = inner.y.saturating_add((i % rows) as u16);
        if y >= inner.bottom() || x >= inner.right() {
            continue;
        }
        let line = Line::from(vec![
            Span::styled(format!(" {key}"), theme.highlight()),
            Span::styled(" → ", theme.muted()),
            Span::styled(label.clone(), theme.text()),
        ]);
        let width = (entry_width as u16).min(inner.right() - x);
        frame.render_widget(line, Rect::new(x, y, width, 1));
    }
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
    // Wipe the backdrop first: the surface sets only a background, so the
    // backdrop's muted foreground would stay under every line of text.
    frame.render_widget(Clear, dialog.rect);
    let border = theme.border_set(None);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border)
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
    // The resize grip is the frame's own corner, highlighted.
    frame.render_widget(
        Paragraph::new(border.bottom_right).style(theme.highlight()),
        grip,
    );
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
        let line = Line::from(crate::widgets::pill(
            &theme.glyphs.tab_caps,
            format!(" {title} "),
            theme.tab(active),
            theme,
        ));
        frame.render_widget(Paragraph::new(line), *area);
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
            menu.render_dropdown(&w.id, state, rect, frame, session);
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
                // Hyperlink sequences take no cells on a terminal.
                Some(cell) if !cell.symbol().is_empty() => {
                    line.push_str(&crate::widgets::strip_osc(cell.symbol()));
                }
                _ => line.push(' '),
            }
        }
        output.push_str(line.trim_end());
        output.push('\n');
    }
    Ok(output)
}
