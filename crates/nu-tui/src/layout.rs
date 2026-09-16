//! Screen layout: chrome slots at the top level, then a recursive walk of
//! the visible widget tree. `Split` containers divide their area between
//! their children; other containers stack their children vertically.

use super::app::TuiApp;
use super::session::Session;
use super::widget::{Slot, SplitDir, Widget, WidgetKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SplitterHandle {
    /// Id of the `Split` container this handle belongs to.
    pub id: String,
    pub area: Rect,
    pub direction: SplitDir,
    /// Full region being split (not just the 1-cell handle).
    pub split_area: Rect,
}

/// Mutable layout outputs, borrowed disjointly from the session so the
/// widget tree can be read while they are written.
struct LayoutCtx<'a> {
    areas: &'a mut HashMap<String, Rect>,
    handles: &'a mut Vec<SplitterHandle>,
    ratios: &'a mut HashMap<String, u16>,
}

/// Assign a rectangle to every visible widget. Also records splitter handles
/// and tab titles for mouse hit-testing.
pub fn assign_areas(session: &mut Session, frame: Rect) {
    session.areas.clear();
    session.splitter_handles.clear();
    session.tab_areas.clear();

    let pages = session.pages();
    let page_index = session.page;
    let show_tabs = session.has_tabs();

    let mut constraints: Vec<Constraint> = Vec::new();
    let mut slots: Vec<TopSlot> = Vec::new();
    for w in &session.app.widgets {
        match &w.kind {
            WidgetKind::Label {
                slot: Slot::Title, ..
            }
            | WidgetKind::Menu { .. } => {
                constraints.push(Constraint::Length(1));
                slots.push(TopSlot::Widget(w.id.clone()));
            }
            WidgetKind::Search { .. } => {
                constraints.push(Constraint::Length(3));
                slots.push(TopSlot::Widget(w.id.clone()));
            }
            _ => {}
        }
    }
    if show_tabs {
        constraints.push(Constraint::Length(1));
        slots.push(TopSlot::TabBar);
    }
    constraints.push(Constraint::Min(3));
    slots.push(TopSlot::Content);
    for w in &session.app.widgets {
        if let WidgetKind::Label {
            slot: Slot::Status, ..
        } = &w.kind
        {
            constraints.push(Constraint::Length(1));
            slots.push(TopSlot::Widget(w.id.clone()));
        }
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame);

    let mut content_area = frame;
    for (slot, rect) in slots.into_iter().zip(chunks.iter()) {
        match slot {
            TopSlot::Widget(id) => {
                session.areas.insert(id, *rect);
            }
            TopSlot::TabBar => assign_tab_areas(session, *rect, pages.len()),
            TopSlot::Content => content_area = *rect,
        }
    }

    // Root content: bare (non-chrome, non-tab) roots plus the active tab.
    let active_tab = pages.get(page_index).and_then(|p| p.id.clone());
    let Session {
        app,
        areas,
        splitter_handles,
        splitter_ratio,
        ..
    } = session;
    let mut ctx = LayoutCtx {
        areas,
        handles: splitter_handles,
        ratios: splitter_ratio,
    };
    let content: Vec<&Widget> = app
        .widgets
        .iter()
        .filter(|w| match &w.kind {
            WidgetKind::Tab { .. } => Some(&w.id) == active_tab.as_ref(),
            k => !k.is_chrome(),
        })
        .collect();
    // A root tab is a page: its children fill the content area directly.
    let nodes: Vec<&Widget> = content
        .into_iter()
        .flat_map(|w| match &w.kind {
            WidgetKind::Tab { .. } => {
                ctx.areas.insert(w.id.clone(), content_area);
                w.children.iter().collect::<Vec<_>>()
            }
            _ => vec![w],
        })
        .collect();
    layout_nodes(&mut ctx, &nodes, content_area);
}

enum TopSlot {
    Widget(String),
    TabBar,
    Content,
}

fn assign_tab_areas(session: &mut Session, bar: Rect, n_pages: usize) {
    let n = n_pages.max(1) as u16;
    let width = bar.width.checked_div(n).unwrap_or(bar.width);
    for p in 0..n_pages {
        session.tab_areas.push(Rect {
            x: bar.x + p as u16 * width,
            y: bar.y,
            width,
            height: 1,
        });
    }
}

/// Stack sibling widgets vertically.
fn layout_nodes(ctx: &mut LayoutCtx, nodes: &[&Widget], area: Rect) {
    if nodes.is_empty() {
        return;
    }
    let constraints: Vec<Constraint> = nodes
        .iter()
        .map(|w| match &w.kind {
            WidgetKind::Label { .. } => Constraint::Length(1),
            WidgetKind::TextBox { .. } | WidgetKind::Search { .. } => Constraint::Length(3),
            WidgetKind::Menu { .. } => Constraint::Length(1),
            WidgetKind::Table { .. }
            | WidgetKind::Log { .. }
            | WidgetKind::Tree { .. }
            | WidgetKind::Preview { .. }
            | WidgetKind::Split { .. }
            | WidgetKind::Tab { .. } => Constraint::Min(5),
        })
        .collect();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    for (w, rect) in nodes.iter().zip(chunks.iter()) {
        assign_node(ctx, w, *rect);
    }
}

fn assign_node(ctx: &mut LayoutCtx, w: &Widget, area: Rect) {
    ctx.areas.insert(w.id.clone(), area);
    match &w.kind {
        WidgetKind::Split { direction, ratio } => layout_split(ctx, w, *direction, *ratio, area),
        WidgetKind::Tab { .. } => {
            // Nested tab: a titled group box around its children.
            let inner = Block::default().borders(Borders::ALL).inner(area);
            let kids: Vec<&Widget> = w.children.iter().collect();
            layout_nodes(ctx, &kids, inner);
        }
        _ => {}
    }
}

fn layout_split(ctx: &mut LayoutCtx, w: &Widget, direction: SplitDir, ratio: u16, area: Rect) {
    let n = w.children.len();
    if n == 0 {
        return;
    }
    if n == 1 {
        assign_node(ctx, &w.children[0], area);
        return;
    }
    let permille = *ctx
        .ratios
        .entry(w.id.clone())
        .or_insert_with(|| percent_to_permille(ratio));
    let permille = permille.clamp(50, 950);
    let (first, handle, rest) = split_axis(area, direction, permille);
    assign_node(ctx, &w.children[0], first);
    if let Some(handle) = handle {
        ctx.handles.push(SplitterHandle {
            id: w.id.clone(),
            area: handle,
            direction,
            split_area: area,
        });
    }
    if n == 2 {
        assign_node(ctx, &w.children[1], rest);
        return;
    }
    // More than two children: the remainder is shared equally on the same axis.
    let layout_dir = match direction {
        SplitDir::Horizontal => Direction::Horizontal,
        SplitDir::Vertical => Direction::Vertical,
    };
    let share = (100 / (n - 1)) as u16;
    let chunks = Layout::default()
        .direction(layout_dir)
        .constraints(vec![Constraint::Percentage(share); n - 1])
        .split(rest);
    for (child, rect) in w.children[1..].iter().zip(chunks.iter()) {
        assign_node(ctx, child, *rect);
    }
}

pub fn percent_to_permille(percent: u16) -> u16 {
    (percent.clamp(10, 90) as u32 * 10) as u16
}

/// Length of the first pane and whether a 1-cell handle fits.
/// `u16::clamp` panics when min > max, so thin splits skip the handle.
fn split_lengths(total: u16, ratio: u16) -> (u16, bool) {
    if total <= 1 {
        return (total, false);
    }
    let first = ((total as u32 * ratio as u32) / 1000) as u16;
    if total < 7 {
        let max = total.saturating_sub(1);
        return (first.clamp(1, max), false);
    }
    let max = total.saturating_sub(4);
    let min = 3.min(max);
    (first.clamp(min, max), true)
}

fn split_axis(area: Rect, direction: SplitDir, ratio: u16) -> (Rect, Option<Rect>, Rect) {
    let (layout_dir, total) = match direction {
        SplitDir::Horizontal => (Direction::Horizontal, area.width),
        SplitDir::Vertical => (Direction::Vertical, area.height),
    };
    if total == 0 {
        return (area, None, area);
    }
    let (first, with_handle) = split_lengths(total, ratio);
    if with_handle {
        let chunks = Layout::default()
            .direction(layout_dir)
            .constraints([
                Constraint::Length(first),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(area);
        let a = chunks.first().copied().unwrap_or(area);
        let handle = chunks.get(1).copied();
        let b = chunks.get(2).copied().unwrap_or(a);
        (a, handle, b)
    } else {
        let chunks = Layout::default()
            .direction(layout_dir)
            .constraints([Constraint::Length(first), Constraint::Min(1)])
            .split(area);
        let a = chunks.first().copied().unwrap_or(area);
        let b = chunks.get(1).copied().unwrap_or(a);
        (a, None, b)
    }
}

/// Ids in the subtree of `root`, preorder. Used to scope searches and to
/// find the nearest row source for a preview.
pub fn subtree_ids(app: &TuiApp, path: &[usize]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(w) = app.at_path(path) {
        push_ids(w, &mut out);
    }
    out
}

fn push_ids(w: &Widget, out: &mut Vec<String>) {
    out.push(w.id.clone());
    for c in &w.children {
        push_ids(c, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thin_split_does_not_panic() {
        for total in 0..12u16 {
            let _ = split_lengths(total, 500);
        }
        let (first, handle) = split_lengths(4, 500);
        assert!(!handle);
        assert!((1..4).contains(&first));
        let (first, handle) = split_lengths(20, 500);
        assert!(handle);
        assert!((3..=16).contains(&first));
    }
}
