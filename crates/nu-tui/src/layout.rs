//! Screen layout: chrome slots at the top level, then a recursive walk of
//! the visible widget tree. `Split` containers divide their area between
//! their children with a one-cell handle between each pair; other
//! containers stack their children vertically.

use crate::app::TuiApp;
use crate::session::Session;
use crate::widget::{Widget, WidgetKind, WidgetState};
use crate::widgets::label::Slot;
use crate::widgets::split::SplitDir;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};
use std::collections::HashMap;

/// The one-cell gap between two split children, for mouse dragging.
#[derive(Debug, Clone)]
pub struct SplitterHandle {
    /// Id of the `Split` container this handle belongs to.
    pub id: String,
    /// Index of the child on the near side of the handle.
    pub index: usize,
    pub area: Rect,
    pub direction: SplitDir,
    /// Full region being split (not just the 1-cell handle).
    pub split_area: Rect,
    /// Offset of that child's start from the split's origin, along the axis.
    pub child_start: u16,
}

/// Mutable layout outputs, borrowed disjointly from the session so the
/// widget tree can be read while they are written.
struct LayoutCtx<'a> {
    areas: &'a mut HashMap<String, Rect>,
    handles: &'a mut Vec<SplitterHandle>,
    states: &'a HashMap<String, WidgetState>,
}

/// Assign a rectangle to every visible widget. Also records splitter handles
/// and tab titles for mouse hit-testing.
pub fn assign_areas(session: &mut Session, frame: Rect) {
    session.areas.clear();
    session.handles.clear();
    session.tab_areas.clear();

    let pages = session.pages();
    let page_index = session.page;
    let show_tabs = session.has_tabs();

    let mut constraints: Vec<Constraint> = Vec::new();
    let mut slots: Vec<TopSlot> = Vec::new();
    for w in &session.app.widgets {
        match &w.kind {
            WidgetKind::Label(l) if l.slot == Slot::Title => {
                constraints.push(w.kind.constraint());
                slots.push(TopSlot::Widget(w.id.clone()));
            }
            WidgetKind::Menu(_) | WidgetKind::Search(_) => {
                constraints.push(w.kind.constraint());
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
        if let WidgetKind::Label(l) = &w.kind
            && l.slot == Slot::Status
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
        handles,
        states,
        ..
    } = session;
    let mut ctx = LayoutCtx {
        areas,
        handles,
        states,
    };
    let content: Vec<&Widget> = app
        .widgets
        .iter()
        .filter(|w| match &w.kind {
            WidgetKind::Tab(_) => Some(&w.id) == active_tab.as_ref(),
            k => !k.is_chrome(),
        })
        .collect();
    // A root tab is a page: its children fill the content area directly.
    let nodes: Vec<&Widget> = content
        .into_iter()
        .flat_map(|w| match &w.kind {
            WidgetKind::Tab(_) => {
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

/// Consecutive flowing siblings (buttons) share one row; every other widget
/// is a row of its own.
fn rows_of<'a>(nodes: &[&'a Widget]) -> Vec<Vec<&'a Widget>> {
    let mut rows: Vec<Vec<&Widget>> = Vec::new();
    for w in nodes {
        let flows = w.kind.caps().flows;
        match rows.last_mut() {
            Some(row) if flows && row.iter().all(|r| r.kind.caps().flows) => row.push(w),
            _ => rows.push(vec![w]),
        }
    }
    rows
}

/// Height of a widget that never grows: a fixed leaf, a flow row, or a
/// container whose children are all fixed. `None` means it fills.
pub fn fixed_height(w: &Widget) -> Option<u16> {
    match &w.kind {
        WidgetKind::Split(split) => {
            let heights = w
                .children
                .iter()
                .map(fixed_height)
                .collect::<Option<Vec<u16>>>()?;
            match split.direction {
                SplitDir::Horizontal => heights.into_iter().max(),
                SplitDir::Vertical => Some(heights.into_iter().sum()),
            }
        }
        WidgetKind::Box(_) => {
            let kids: Vec<&Widget> = w.children.iter().collect();
            let heights = rows_of(&kids)
                .iter()
                .map(|row| row_fixed_height(row))
                .collect::<Option<Vec<u16>>>()?;
            Some(heights.into_iter().sum::<u16>() + 2)
        }
        _ => match w.kind.constraint() {
            Constraint::Length(n) => Some(n),
            _ => None,
        },
    }
}

fn row_fixed_height(row: &[&Widget]) -> Option<u16> {
    row.iter()
        .map(|w| fixed_height(w))
        .collect::<Option<Vec<u16>>>()?
        .into_iter()
        .max()
}

fn row_constraint(row: &[&Widget]) -> Constraint {
    match row_fixed_height(row) {
        Some(n) => Constraint::Length(n),
        None => row
            .first()
            .map(|w| w.kind.constraint())
            .unwrap_or(Constraint::Min(1)),
    }
}

/// Stack sibling widgets vertically. Flowing siblings pack left to right on
/// one row, each as wide as its `width_hint`.
fn layout_nodes(ctx: &mut LayoutCtx, nodes: &[&Widget], area: Rect) {
    if nodes.is_empty() {
        return;
    }
    let rows = rows_of(nodes);
    let constraints: Vec<Constraint> = rows.iter().map(|row| row_constraint(row)).collect();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    for (row, rect) in rows.iter().zip(chunks.iter()) {
        if let [single] = row.as_slice() {
            assign_node(ctx, single, *rect);
            continue;
        }
        let mut widths: Vec<Constraint> = row
            .iter()
            .map(|w| Constraint::Length(w.kind.width_hint().unwrap_or(10)))
            .collect();
        widths.push(Constraint::Fill(1));
        let cells = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(widths)
            .split(*rect);
        for (w, cell) in row.iter().zip(cells.iter()) {
            assign_node(ctx, w, *cell);
        }
    }
}

fn assign_node(ctx: &mut LayoutCtx, w: &Widget, area: Rect) {
    ctx.areas.insert(w.id.clone(), area);
    match &w.kind {
        WidgetKind::Split(split) => {
            let n = w.children.len();
            if n == 0 {
                return;
            }
            let sizes = ctx
                .states
                .get(&w.id)
                .and_then(WidgetState::as_split)
                .map(|s| s.sizes.clone())
                .filter(|s| s.len() == n)
                .unwrap_or_else(|| split.sizes_for(n));
            layout_split(ctx, w, split.direction, &sizes, area);
        }
        WidgetKind::Box(_) => {
            let inner = Block::default().borders(Borders::ALL).inner(area);
            let kids: Vec<&Widget> = w.children.iter().collect();
            layout_nodes(ctx, &kids, inner);
        }
        _ => {}
    }
}

fn layout_split(
    ctx: &mut LayoutCtx,
    w: &Widget,
    direction: SplitDir,
    sizes: &[crate::widget::Size],
    area: Rect,
) {
    let layout_dir = match direction {
        SplitDir::Horizontal => Direction::Horizontal,
        SplitDir::Vertical => Direction::Vertical,
    };
    let constraints: Vec<Constraint> = sizes.iter().map(|s| s.to_constraint()).collect();
    // A split of fixed-height leaves (a button row, a form line) is not
    // resizable: no gap and no handle between its children.
    let fixed = fixed_height(w).is_some();
    let (segments, spacers) = Layout::default()
        .direction(layout_dir)
        .constraints(constraints)
        .spacing(if fixed { 0 } else { 1 })
        .split_with_spacers(area);
    for (child, rect) in w.children.iter().zip(segments.iter()) {
        assign_node(ctx, child, *rect);
    }
    if fixed {
        return;
    }
    // Spacers include the outer edges; the inner ones are the handles.
    for (index, spacer) in spacers
        .iter()
        .skip(1)
        .take(w.children.len() - 1)
        .enumerate()
    {
        if spacer.width == 0 || spacer.height == 0 {
            continue;
        }
        let child_start = segments
            .get(index)
            .map(|r| match direction {
                SplitDir::Horizontal => r.x.saturating_sub(area.x),
                SplitDir::Vertical => r.y.saturating_sub(area.y),
            })
            .unwrap_or(0);
        ctx.handles.push(SplitterHandle {
            id: w.id.clone(),
            index,
            area: *spacer,
            direction,
            split_area: area,
            child_start,
        });
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
