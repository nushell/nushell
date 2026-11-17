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
    // A split of fixed-height leaves (a button row, a form line) is not
    // resizable: no gap and no handle between its children.
    let fixed = fixed_height(w).is_some();
    let (segments, spacers) = direction.split(sizes, !fixed, area);
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
        ctx.handles.push(SplitterHandle {
            id: w.id.clone(),
            index,
            area: *spacer,
            direction,
            split_area: area,
        });
    }
}

/// How many `Fill` constraints [`pin_fills`] leaves to ratatui's solver.
/// kasuari, the solver, picks pivots in hash order with no rule against
/// cycling, and the more `Fill`s it gets, the likelier a solve never ends:
/// at 40 equal ones, none of 10 release-build runs finished within 5 s,
/// while at 28 all 40 runs finished within 60 ms. 16 is a margin below 28,
/// not a guarantee: in an area too narrow for the other constraints, the
/// solver can stall with fewer `Fill`s.
const MAX_SOLVER_FILLS: usize = 16;

/// `constraints` ready for ratatui's solver on a line of `length` cells,
/// `spacing` cells apart, under the default `Flex::Start`.
///
/// Up to [`MAX_SOLVER_FILLS`] `Fill`s come back as they are. Past it, each
/// `Fill` becomes a `Length` of its share of the cells the solver gives all
/// of them as one `Fill`. Beside `Length` and `Max` constraints those are
/// the cells the solver would give each `Fill`. Beside a `Percentage` a
/// `Fill` can be a cell off, and beside a `Min` the `Fill`s and the `Min`
/// can be up to two cells off.
pub fn pin_fills(constraints: Vec<Constraint>, spacing: u16, length: u16) -> Vec<Constraint> {
    pin_fills_past(MAX_SOLVER_FILLS, constraints, spacing, length)
}

/// [`pin_fills`] past `max_fills` `Fill`s; tests lower it to compare the
/// pinned cells with the solver's own.
fn pin_fills_past(
    max_fills: usize,
    constraints: Vec<Constraint>,
    spacing: u16,
    length: u16,
) -> Vec<Constraint> {
    let weight = |c: &Constraint| match c {
        Constraint::Fill(w) => Some(u64::from(*w)),
        _ => None,
    };
    let fills = constraints.iter().filter_map(weight).count();
    let Some(last) = constraints
        .iter()
        .rposition(|c| weight(c).is_some())
        .filter(|_| fills > max_fills)
    else {
        return constraints;
    };
    let total: u64 = constraints.iter().filter_map(weight).sum();
    // First pass: the `Fill`s as one, in the last one's place. It weighs
    // their sum because under `Flex::Start` a `Min` shares space with the
    // `Fill`s by weight.
    let group = Constraint::Fill(u16::try_from(total).unwrap_or(u16::MAX));
    let merged = constraints
        .iter()
        .enumerate()
        .filter_map(|(i, &c)| match c {
            Constraint::Fill(_) => (i == last).then_some(group),
            other => Some(other),
        });
    let segments = Layout::horizontal(merged)
        .spacing(spacing)
        .split(Rect::new(0, 0, length, 1));
    let index = constraints[..last]
        .iter()
        .filter(|c| weight(c).is_none())
        .count();
    // The second pass also puts a spacer between the `Fill`s.
    let spacers = u16::try_from(fills - 1)
        .unwrap_or(u16::MAX)
        .saturating_mul(spacing);
    let cells = segments
        .get(index)
        .map_or(0, |rect| rect.width.saturating_sub(spacers));
    let cells = u64::from(cells);
    // Second pass: each `Fill` pinned to its share. As in the solver, a
    // `0fr` gets nothing beside weighted `Fill`s, and `0fr`s alone share
    // evenly. Rounding the running total, as the solver rounds edges, keeps
    // the sum exact.
    let parts = if total == 0 { fills as u64 } else { total };
    let mut done = 0;
    let mut start = 0;
    constraints
        .into_iter()
        .map(|c| {
            let Some(w) = weight(&c) else {
                return c;
            };
            done += if total == 0 { 1 } else { w };
            let end = (2 * cells * done + parts) / (2 * parts);
            let share = end - start;
            start = end;
            Constraint::Length(u16::try_from(share).unwrap_or(u16::MAX))
        })
        .collect()
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
    use Constraint::{Fill, Length, Max};

    #[test]
    fn many_fills_reach_the_solver_as_lengths() {
        // 40 `1fr` a cell apart in 400 cells: 361 cells, 9.025 each. The
        // running total rounds up at the 20th.
        let mut want = vec![Length(9); 40];
        want[19] = Length(10);
        assert_eq!(pin_fills(vec![Fill(1); 40], 1, 400), want);
        // Up to the limit the solver gets them as they are.
        let fills = vec![Fill(1); MAX_SOLVER_FILLS];
        assert_eq!(pin_fills(fills.clone(), 1, 400), fills);
    }

    #[test]
    fn fill_group_weighs_against_a_min() {
        // Under `Flex::Start` a `Min` grows like a `1fr`: beside 17 `1fr` it
        // gets 1/18 of 400 cells, as the solver gives it beside 17 `Fill`s.
        let mut constraints = vec![Fill(1); 17];
        constraints.push(Constraint::Min(5));
        let pinned = pin_fills(constraints, 0, 400);
        let cells: u16 = pinned
            .iter()
            .map(|c| match c {
                Length(n) => *n,
                _ => 0,
            })
            .sum();
        assert_eq!(cells, 378, "{pinned:?}");
    }

    #[test]
    fn pinned_fills_get_the_solvers_cells() {
        // Pinned even below the limit, the `Fill`s get the cells the solver
        // gives them beside `Length` and `Max` siblings. Few `Fill`s on wide
        // lines, and no `0fr`, so the solver always finishes.
        let weights: [fn(usize) -> u16; 3] = [
            |_| 1,
            |i| if i % 2 == 0 { 1 } else { 2 },
            |i| if i == 0 { 3 } else { 1 },
        ];
        for (front, back) in [(None, None), (Some(Length(20)), None), (None, Some(Max(5)))] {
            for weight in weights {
                for n in 2..=5 {
                    let constraints: Vec<Constraint> = front
                        .into_iter()
                        .chain((0..n).map(|i| Fill(weight(i))))
                        .chain(back)
                        .collect();
                    for spacing in [0, 1] {
                        for length in [50, 79, 100, 201] {
                            let area = Rect::new(0, 0, length, 1);
                            let layout =
                                |c: Vec<Constraint>| Layout::horizontal(c).spacing(spacing);
                            let pinned = pin_fills_past(0, constraints.clone(), spacing, length);
                            assert_eq!(
                                layout(pinned).split_with_spacers(area),
                                layout(constraints.clone()).split_with_spacers(area),
                                "{constraints:?} on {length}, spacing {spacing}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn zero_weight_fills_pin_like_the_solver() {
        // Beside weighted `Fill`s a `0fr` gets nothing; `0fr`s alone share
        // evenly.
        let pinned = pin_fills_past(0, vec![Fill(0), Fill(1), Fill(1)], 0, 10);
        assert_eq!(pinned, [Length(0), Length(5), Length(5)]);
        let pinned = pin_fills_past(0, vec![Fill(0), Fill(0)], 0, 10);
        assert_eq!(pinned, [Length(5), Length(5)]);
    }
}
