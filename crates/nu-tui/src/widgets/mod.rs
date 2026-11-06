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

use crate::session::Session;
use crate::theme::{Caps, Theme};
use ansi_str::get_blocks;
use nu_explore::style::ansi_style_to_tui;
use ratatui::Frame;
use ratatui::buffer::{Buffer, CellDiffOption, CellWidth};
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders};
use std::borrow::Cow;
use std::num::NonZeroU16;
use unicode_width::UnicodeWidthChar;

/// The name at the start of a widget's border title: its `--title`, else
/// `default`. `--title` is never empty (see `push_widget`), so an empty
/// result means the widget has no `--title` and `default` was empty.
pub(crate) fn custom_title<'a>(session: &'a Session, id: &str, default: &'a str) -> &'a str {
    session
        .widget(id)
        .and_then(|w| w.title.as_deref())
        .unwrap_or(default)
}

/// A bordered block with a title, colored for focus. The lines are widget
/// `id`'s `--border`, or `$env.config.tui.border_type`. The title takes the
/// border style too, so focus shows even when the lines are blank
/// (`frameless`).
pub(crate) fn framed(title: &str, focused: bool, session: &Session, id: &str) -> Block<'static> {
    let theme = &session.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme.border_set(session.widget(id).and_then(|w| w.border)))
        .style(theme.surface())
        .border_style(theme.border(focused));
    if title.is_empty() {
        block
    } else {
        block.title(format!(" {title} "))
    }
}

/// Text in `style` between `caps`, each cap in the text's fill color: a
/// pill when the caps are shaped, plain text when they are empty.
pub(crate) fn pill(caps: &Caps, text: String, style: Style, theme: &Theme) -> Vec<Span<'static>> {
    let cap = theme.cap_style(style);
    vec![
        Span::styled(caps.open.clone(), cap),
        Span::styled(text, style),
        Span::styled(caps.close.clone(), cap),
    ]
}

/// The room for a list's rows: `inner` less a cell at each side when
/// `selected_caps` are set, where [`draw_selected_caps`] draws them.
pub(crate) fn capped(inner: Rect, theme: &Theme) -> Rect {
    if theme.glyphs.selected_caps.is_some() {
        inner.inner(Margin::new(1, 0))
    } else {
        inner
    }
}

/// Draw `selected_caps` beside row `y` of a list laid out in `inner` (from
/// [`capped`]), in the fill the row shows highlighted: `row` is its style
/// before `selected` goes over it.
pub(crate) fn draw_selected_caps(
    frame: &mut Frame,
    inner: Rect,
    y: u16,
    row: Style,
    theme: &Theme,
) {
    let Some(caps) = &theme.glyphs.selected_caps else {
        return;
    };
    if y < inner.top() || y >= inner.bottom() || inner.width == 0 {
        return;
    }
    let style = theme.cap_style(row.patch(theme.selected()));
    let buf = frame.buffer_mut();
    buf.set_string(inner.left().saturating_sub(1), y, &caps.open, style);
    buf.set_string(inner.right(), y, &caps.close, style);
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
    linked_line(line, theme).0
}

/// A hyperlink in a line: the columns it covers and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Link {
    pub start: usize,
    pub end: usize,
    pub url: String,
}

/// `line` as styled spans, with the hyperlinks (OSC 8) it carries. The
/// link sequences leave the text; the columns they covered are returned so
/// a widget drawing the line can link the cells (see [`hyperlink_cells`]).
pub(crate) fn linked_line(line: &str, theme: &Theme) -> (Line<'static>, Vec<Link>) {
    let mut spans = Vec::new();
    let mut links = Vec::new();
    let mut open: Option<(usize, String)> = None;
    let mut column = 0;
    let mut close = |open: &mut Option<(usize, String)>, column: usize| {
        if let Some((start, url)) = open.take()
            && start < column
        {
            links.push(Link {
                start,
                end: column,
                url,
            });
        }
    };
    // A bare `ESC [ m` resets, as `0` does, but `ansi-str` takes it for an
    // unknown attribute and keeps the old ones. glow resets this way.
    let line = line.replace("\x1b[m", "\x1b[0m");
    for block in get_blocks(&line) {
        let style = theme.surface().patch(ansi_style_to_tui(block.style()));
        for piece in osc_pieces(block.text()) {
            match piece {
                Piece::Text(text) => {
                    column += columns(text);
                    spans.push(Span::styled(text.to_string(), style));
                }
                // `8;<params>;<url>`: a URL opens a link, an empty one closes it.
                Piece::Osc(osc) => {
                    if let Some(params) = osc.strip_prefix("8;") {
                        close(&mut open, column);
                        let url = params.split_once(';').map_or("", |(_, url)| url);
                        if !url.is_empty() {
                            open = Some((column, url.to_string()));
                        }
                    }
                }
            }
        }
    }
    close(&mut open, column);
    let line = if spans.is_empty() {
        Line::from(Span::styled(String::new(), theme.surface()))
    } else {
        Line::from(spans)
    };
    (line, links)
}

/// Columns `text` takes, measured as [`wrap_line`] measures.
pub(crate) fn columns(text: &str) -> usize {
    text.chars().map(|c| c.width().unwrap_or(0)).sum()
}

/// Mark the cells of a drawn `row` that `links` cover as hyperlinks: the
/// OSC 8 sequence opens inside the first cell's symbol and closes inside
/// the last's, with the cells' widths pinned so the sequences do not count.
/// ratatui has no hyperlink style; its own buffer tests link cells this way.
/// `first` is the column of the row's first cell within its unwrapped line.
pub(crate) fn hyperlink_cells(buf: &mut Buffer, row: Rect, first: usize, links: &[Link]) {
    let last = first + row.width as usize;
    for link in links {
        let (start, end) = (link.start.max(first), link.end.min(last));
        if start >= end {
            continue;
        }
        // Where the cells are; the last one is the start of the last
        // symbol, which may be a wide character spanning two columns.
        let x = |column: usize| row.x + (column - first) as u16;
        let mut column = start;
        let mut last_cell = start;
        while column < end {
            last_cell = column;
            column += buf
                .cell((x(column), row.y))
                .map_or(1, |cell| cell.cell_width().max(1) as usize);
        }
        wrap_symbol(
            buf,
            (x(start), row.y),
            &format!("\x1b]8;;{}\x1b\\", link.url),
            "",
        );
        wrap_symbol(buf, (x(last_cell), row.y), "", "\x1b]8;;\x1b\\");
    }
}

/// Put `before` and `after` around a cell's symbol without changing the
/// width the cell is diffed and drawn with.
fn wrap_symbol(buf: &mut Buffer, at: (u16, u16), before: &str, after: &str) {
    let Some(cell) = buf.cell_mut(at) else { return };
    let Some(width) = NonZeroU16::new(cell.cell_width().max(1)) else {
        return;
    };
    let symbol = format!("{before}{}{after}", cell.symbol());
    cell.set_symbol(&symbol)
        .set_diff_option(CellDiffOption::ForcedWidth(width));
}

/// A piece of a line: text, or the body of an OSC sequence (`ESC ]` up to
/// `BEL` or `ESC \`). `ansi-str` reads only SGR sequences and leaves OSC
/// ones in the text, which is how glow's links came out as text.
enum Piece<'a> {
    Text(&'a str),
    Osc(&'a str),
}

fn osc_pieces(text: &str) -> impl Iterator<Item = Piece<'_>> {
    const OSC: &str = "\x1b]";
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let piece = match rest.strip_prefix(OSC) {
            Some(body) => {
                let (osc, after) = match body.find(['\x07', '\x1b']) {
                    Some(at) if body[at..].starts_with("\x1b\\") => (&body[..at], &body[at + 2..]),
                    Some(at) if body[at..].starts_with('\x07') => (&body[..at], &body[at + 1..]),
                    // Another sequence starts: this one ends there.
                    Some(at) => (&body[..at], &body[at..]),
                    // Unterminated: the sequence swallows the rest of the line.
                    None => (body, ""),
                };
                rest = after;
                Piece::Osc(osc)
            }
            None => {
                let (text, after) = rest.split_at(rest.find(OSC).unwrap_or(rest.len()));
                rest = after;
                Piece::Text(text)
            }
        };
        Some(piece)
    })
}

/// `text` without its OSC sequences, as a terminal would show it.
pub(crate) fn strip_osc(text: &str) -> Cow<'_, str> {
    if !text.contains("\x1b]") {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        osc_pieces(text)
            .filter_map(|piece| match piece {
                Piece::Text(text) => Some(text),
                Piece::Osc(_) => None,
            })
            .collect(),
    )
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
            let w = columns(c.encode_utf8(&mut [0; 4]));
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
    fn osc_sequences_are_dropped_but_their_text_stays() {
        let bel = "see \x1b]8;id=1;http://a\x07link\x1b]8;;\x07 here";
        assert_eq!(strip_osc(bel), "see link here");
        let st = "\x1b]8;;http://a\x1b\\link\x1b]8;;\x1b\\";
        assert_eq!(strip_osc(st), "link");
        assert_eq!(strip_osc("\x1b[1mbold\x1b[0m"), "\x1b[1mbold\x1b[0m");
        assert_eq!(strip_osc("cut \x1b]8;;http://a"), "cut ");
        assert_eq!(strip_osc("\x1b]0;title\x1b[1mx"), "\x1b[1mx");
    }

    #[test]
    fn links_keep_their_columns_through_styling() {
        let theme = Theme::default();
        let raw = "\x1b[1msee \x1b]8;id=1;http://a\x07li\x1b[0mnk\x1b]8;;\x07 and \x1b]8;;http://b\x1b\\b\x1b]8;;\x1b\\";
        let (line, links) = linked_line(raw, &theme);
        assert_eq!(line.to_string(), "see link and b");
        assert_eq!(
            links,
            [
                Link {
                    start: 4,
                    end: 8,
                    url: "http://a".into()
                },
                Link {
                    start: 13,
                    end: 14,
                    url: "http://b".into()
                },
            ]
        );
        // A bare reset ends bold, as a terminal would have it.
        let line = ansi_line("\x1b[1mbold\x1b[m plain", &theme);
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert!(!line.spans[1].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(line.to_string(), "bold plain");
        // A link with no text, and one left open, still end cleanly.
        let (line, links) = linked_line(
            "\x1b]8;;http://a\x07\x1b]8;;\x07x\x1b]8;;http://b\x07yz",
            &theme,
        );
        assert_eq!(line.to_string(), "xyz");
        assert_eq!(
            links,
            [Link {
                start: 1,
                end: 3,
                url: "http://b".into()
            }]
        );
    }

    #[test]
    fn linked_cells_carry_the_sequence_in_their_edges() {
        let theme = Theme::default();
        let (line, links) = linked_line("ab \x1b]8;;http://a\x07link\x1b]8;;\x07 c", &theme);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        buf.set_line(0, 0, &line, 10);
        hyperlink_cells(&mut buf, Rect::new(0, 0, 10, 1), 0, &links);
        assert_eq!(buf[(3, 0)].symbol(), "\x1b]8;;http://a\x1b\\l");
        assert_eq!(buf[(4, 0)].symbol(), "i");
        assert_eq!(buf[(6, 0)].symbol(), "k\x1b]8;;\x1b\\");
        assert_eq!(buf[(3, 0)].cell_width(), 1);
        assert_eq!(buf[(6, 0)].cell_width(), 1);

        // Wrapped: the link covers columns 3..7 of the line, the second
        // row holds columns 5.. of it, so "nk" links on that row.
        let rows = wrap_line(&line, 5);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 2));
        buf.set_line(0, 1, &rows[1], 5);
        hyperlink_cells(&mut buf, Rect::new(0, 1, 5, 1), 5, &links);
        assert_eq!(buf[(0, 1)].symbol(), "\x1b]8;;http://a\x1b\\n");
        assert_eq!(buf[(1, 1)].symbol(), "k\x1b]8;;\x1b\\");

        // A wide character at the end carries the close; its width stays 2.
        let (line, links) = linked_line("\x1b]8;;http://a\x07a\u{1F517}\x1b]8;;\x07", &theme);
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.set_line(0, 0, &line, 4);
        hyperlink_cells(&mut buf, Rect::new(0, 0, 4, 1), 0, &links);
        assert_eq!(buf[(1, 0)].symbol(), "\u{1F517}\x1b]8;;\x1b\\");
        assert_eq!(buf[(1, 0)].cell_width(), 2);
    }

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
