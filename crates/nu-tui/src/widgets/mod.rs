//! One module per widget kind. Each implements [`TuiWidget`](crate::widget::TuiWidget)
//! for its definition struct and keeps its own key, mouse, render, and result
//! logic, so the session only orchestrates.

pub mod r#box;
pub mod button;
pub mod label;
pub mod log;
pub mod menu;
pub mod preview;
pub mod progress;
pub mod search;
pub mod select;
pub mod split;
pub mod tab;
pub mod table;
pub mod textbox;
pub mod tree;

use crate::theme::Theme;
use ansi_str::get_blocks;
use nu_explore::style::ansi_style_to_tui;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders};

/// A bordered block with a title, colored for focus.
pub(crate) fn framed(title: &str, focused: bool, theme: &Theme) -> Block<'static> {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(theme.surface())
        .border_style(theme.border(focused));
    if title.is_empty() {
        block
    } else {
        block.title(format!(" {title} "))
    }
}

/// Text with ANSI escapes (from `nu-highlight`, `ls` colors, …) as styled
/// lines over the surface color.
pub(crate) fn ansi_to_text(text: &str, theme: &Theme) -> Text<'static> {
    Text::from(
        text.split('\n')
            .map(|line| ansi_line(line, theme))
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn ansi_line(line: &str, theme: &Theme) -> Line<'static> {
    let spans: Vec<Span<'static>> = get_blocks(line)
        .map(|block| {
            Span::styled(
                block.text().to_string(),
                theme.surface().patch(ansi_style_to_tui(block.style())),
            )
        })
        .collect();
    if spans.is_empty() {
        Line::from(Span::styled(String::new(), theme.surface()))
    } else {
        Line::from(spans)
    }
}

/// `text` with a block cursor at `cursor` (in characters).
pub(crate) fn with_cursor(text: &str, cursor: usize, style: Style) -> Line<'static> {
    let cursor = cursor.min(text.chars().count());
    let mut chars = text.chars();
    let before: String = chars.by_ref().take(cursor).collect();
    let after: String = chars.collect();
    Line::from(vec![
        Span::styled(before, style),
        Span::styled("▌", style.add_modifier(Modifier::SLOW_BLINK)),
        Span::styled(after, style),
    ])
}

/// Rows of room inside a bordered list widget: the area minus its borders
/// (and header line for tables).
pub(crate) fn inner_rows(area: Option<&ratatui::layout::Rect>, chrome_lines: u16) -> usize {
    area.map(|a| a.height.saturating_sub(chrome_lines) as usize)
        .unwrap_or(10)
        .max(1)
}
