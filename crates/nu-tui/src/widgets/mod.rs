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
use unicode_width::UnicodeWidthChar;

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

/// Columns of room inside a bordered widget.
pub(crate) fn inner_width(area: Option<&ratatui::layout::Rect>) -> usize {
    area.map(|a| a.width.saturating_sub(2) as usize)
        .unwrap_or(80)
        .max(1)
}

/// Split a styled line into rows of at most `width` columns, breaking
/// between characters. An empty line is one row. Widgets that must know
/// exactly how many rows a line takes (a log following its tail) wrap with
/// this instead of ratatui's word wrap.
pub(crate) fn wrap_line(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut used = 0;
    for span in &line.spans {
        let mut text = String::new();
        for c in span.content.chars() {
            let w = c.width().unwrap_or(0);
            if used + w > width && used > 0 {
                if !text.is_empty() {
                    current.push(Span::styled(std::mem::take(&mut text), span.style));
                }
                rows.push(Line::from(std::mem::take(&mut current)));
                used = 0;
            }
            text.push(c);
            used += w;
        }
        if !text.is_empty() {
            current.push(Span::styled(text, span.style));
        }
    }
    if rows.is_empty() || !current.is_empty() {
        rows.push(Line::from(current));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_line_breaks_between_characters() {
        let line = Line::from("abcdefgh");
        let rows = wrap_line(&line, 3);
        let texts: Vec<String> = rows.iter().map(|l| l.to_string()).collect();
        assert_eq!(texts, ["abc", "def", "gh"]);
        assert_eq!(wrap_line(&Line::from(""), 3).len(), 1);
        assert_eq!(wrap_line(&Line::from("abc"), 3).len(), 1);
    }
}
