//! Screen layout: chrome, pages/tabs, placement splits, and splitter panes.

use super::session::{Page, Session};
use super::widget::{Rel, SplitDir, Widget, WidgetKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone)]
pub struct SplitterHandle {
    pub id: String,
    pub area: Rect,
    pub direction: SplitDir,
    /// Full region being split (not just the 1-cell handle).
    pub split_area: Rect,
}

enum LayoutNode {
    Leaf(usize),
    Split {
        dir: SplitDir,
        ratio: u16,
        a: Box<LayoutNode>,
        b: Box<LayoutNode>,
        handle_id: String,
    },
}

/// Assign a rectangle to every visible widget. Also records splitter handles
/// and tab titles for mouse hit-testing.
pub fn assign_areas(session: &mut Session, frame: Rect) {
    session.areas.clear();
    session.splitter_handles.clear();
    session.tab_areas.clear();

    let mut top_constraints: Vec<Constraint> = Vec::new();
    let mut top_ids: Vec<Option<String>> = Vec::new();
    let mut status_id: Option<String> = None;
    let mut tabs_id: Option<String> = None;

    for w in &session.app.widgets {
        match &w.kind {
            WidgetKind::Title { .. } => {
                top_constraints.push(Constraint::Length(1));
                top_ids.push(Some(w.id.clone()));
            }
            WidgetKind::Menu { .. } => {
                top_constraints.push(Constraint::Length(1));
                top_ids.push(Some(w.id.clone()));
            }
            WidgetKind::Search { .. } => {
                top_constraints.push(Constraint::Length(3));
                top_ids.push(Some(w.id.clone()));
            }
            WidgetKind::Tabs => {
                tabs_id = Some(w.id.clone());
            }
            WidgetKind::Status { .. } => {
                status_id = Some(w.id.clone());
            }
            _ => {}
        }
    }

    let page_index = session.page;
    let pages = session.pages();
    let show_tabs = pages.len() > 1 || tabs_id.is_some();
    let page = pages.get(page_index).cloned();

    if show_tabs {
        top_constraints.push(Constraint::Length(1));
        top_ids.push(tabs_id.clone().or(Some("__tabs__".into())));
    }
    top_constraints.push(Constraint::Min(3));
    top_ids.push(None);
    if let Some(id) = status_id {
        top_constraints.push(Constraint::Length(1));
        top_ids.push(Some(id));
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(top_constraints)
        .split(frame);

    let mut body_area = frame;
    for (i, id) in top_ids.iter().enumerate() {
        if let Some(id) = id {
            if id == "__tabs__" {
                let n = pages.len().max(1) as u16;
                let width = if n == 0 {
                    chunks[i].width
                } else {
                    chunks[i].width / n
                };
                for p in 0..pages.len() {
                    session.tab_areas.push(Rect {
                        x: chunks[i].x + p as u16 * width,
                        y: chunks[i].y,
                        width,
                        height: 1,
                    });
                }
            } else if Some(id) == tabs_id.as_ref() {
                session.areas.insert(id.clone(), chunks[i]);
                let n = pages.len().max(1) as u16;
                let width = if n == 0 {
                    chunks[i].width
                } else {
                    chunks[i].width / n
                };
                for p in 0..pages.len() {
                    session.tab_areas.push(Rect {
                        x: chunks[i].x + p as u16 * width,
                        y: chunks[i].y,
                        width,
                        height: 1,
                    });
                }
            } else {
                session.areas.insert(id.clone(), chunks[i]);
            }
        } else {
            body_area = chunks[i];
        }
    }

    let Some(page) = page else {
        return;
    };

    if let Some(body_id) = &page.body_id {
        session.areas.insert(body_id.clone(), body_area);
    }

    layout_page(session, &page, body_area);
}

fn layout_page(session: &mut Session, page: &Page, area: Rect) {
    let content = page.content.clone();
    if content.is_empty() {
        return;
    }

    let has_place = content.iter().any(|i| {
        session
            .app
            .widgets
            .get(*i)
            .is_some_and(|w| w.place.is_some())
    });

    if has_place {
        if let Some(node) = build_place_tree(&session.app.widgets, &content) {
            assign_node(session, &node, area);
            return;
        }
    }

    if let Some(split_idx) = page.splitter {
        layout_splitter(session, split_idx, &content, area);
        return;
    }

    stack_vertical(session, &content, area);
}

fn layout_splitter(session: &mut Session, split_idx: usize, content: &[usize], area: Rect) {
    let (direction, splitter_id) = match session.app.widgets.get(split_idx) {
        Some(w) => match &w.kind {
            WidgetKind::Splitter { direction, .. } => (*direction, w.id.clone()),
            _ => return,
        },
        None => return,
    };
    let ratio = session
        .splitter_ratio
        .get(&splitter_id)
        .copied()
        .unwrap_or(500)
        .clamp(50, 950);
    split_area(session, area, direction, ratio, content, &splitter_id);
}

fn stack_vertical(session: &mut Session, content: &[usize], area: Rect) {
    let mut constraints = Vec::with_capacity(content.len());
    for idx in content {
        let constraint = match session.app.widgets.get(*idx).map(|w| &w.kind) {
            Some(WidgetKind::Label { .. }) => Constraint::Length(1),
            Some(WidgetKind::TextBox { .. }) => Constraint::Length(3),
            Some(
                WidgetKind::Table { .. }
                | WidgetKind::List { .. }
                | WidgetKind::Log { .. }
                | WidgetKind::Tree { .. }
                | WidgetKind::Keybindings { .. }
                | WidgetKind::Preview { .. },
            ) => Constraint::Min(5),
            _ => Constraint::Min(3),
        };
        constraints.push(constraint);
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    for (i, widget_idx) in content.iter().enumerate() {
        if let Some(w) = session.app.widgets.get(*widget_idx)
            && let Some(rect) = chunks.get(i)
        {
            session.areas.insert(w.id.clone(), *rect);
        }
    }
}

fn split_area(
    session: &mut Session,
    area: Rect,
    direction: SplitDir,
    ratio: u16,
    content: &[usize],
    splitter_id: &str,
) {
    let n = content.len();
    if n == 0 {
        return;
    }
    if n == 1 {
        if let Some(w) = session.app.widgets.get(content[0]) {
            session.areas.insert(w.id.clone(), area);
        }
        return;
    }

    let (first, handle, rest) = split_axis(area, direction, ratio);
    if let Some(w) = session.app.widgets.get(content[0]) {
        session.areas.insert(w.id.clone(), first);
    }
    if let Some(handle) = handle {
        session.splitter_handles.push(SplitterHandle {
            id: splitter_id.to_string(),
            area: handle,
            direction,
            split_area: area,
        });
    }
    if n == 2 {
        if let Some(w) = session.app.widgets.get(content[1]) {
            session.areas.insert(w.id.clone(), rest);
        }
    } else {
        stack_vertical(session, &content[1..], rest);
    }
}

fn build_place_tree(widgets: &[Widget], content: &[usize]) -> Option<LayoutNode> {
    use std::collections::HashMap;
    let mut nodes: HashMap<String, LayoutNode> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for &idx in content {
        let w = widgets.get(idx)?;
        nodes.insert(w.id.clone(), LayoutNode::Leaf(idx));
        order.push(w.id.clone());
    }
    for &idx in content {
        let w = widgets.get(idx)?;
        let Some(place) = &w.place else {
            continue;
        };
        if !nodes.contains_key(&place.of) || place.of == w.id {
            continue;
        }
        let Some(child) = nodes.remove(&w.id) else {
            continue;
        };
        let Some(parent) = nodes.remove(&place.of) else {
            nodes.insert(w.id.clone(), child);
            continue;
        };
        order.retain(|id| id != &w.id);
        let ratio = (place.ratio as u16).clamp(10, 90) * 10;
        let handle_id = format!("split-{}-{}", place.of, w.id);
        let node = match place.rel {
            Rel::RightOf => LayoutNode::Split {
                dir: SplitDir::Horizontal,
                ratio,
                a: Box::new(parent),
                b: Box::new(child),
                handle_id,
            },
            Rel::LeftOf => LayoutNode::Split {
                dir: SplitDir::Horizontal,
                ratio: 1000 - ratio,
                a: Box::new(child),
                b: Box::new(parent),
                handle_id,
            },
            Rel::Below => LayoutNode::Split {
                dir: SplitDir::Vertical,
                ratio,
                a: Box::new(parent),
                b: Box::new(child),
                handle_id,
            },
            Rel::Above => LayoutNode::Split {
                dir: SplitDir::Vertical,
                ratio: 1000 - ratio,
                a: Box::new(child),
                b: Box::new(parent),
                handle_id,
            },
        };
        nodes.insert(place.of.clone(), node);
    }
    let mut roots: Vec<LayoutNode> = order
        .into_iter()
        .filter_map(|id| nodes.remove(&id))
        .collect();
    if roots.is_empty() {
        return None;
    }
    let mut root = roots.remove(0);
    for next in roots {
        let handle_id = format!(
            "split-stack-{}-{}",
            node_anchor_id(&root, widgets),
            node_anchor_id(&next, widgets)
        );
        root = LayoutNode::Split {
            dir: SplitDir::Vertical,
            ratio: 500,
            a: Box::new(root),
            b: Box::new(next),
            handle_id,
        };
    }
    Some(root)
}

fn node_anchor_id(node: &LayoutNode, widgets: &[Widget]) -> String {
    match node {
        LayoutNode::Leaf(idx) => widgets
            .get(*idx)
            .map(|w| w.id.clone())
            .unwrap_or_else(|| idx.to_string()),
        LayoutNode::Split { b, .. } => node_anchor_id(b, widgets),
    }
}

fn assign_node(session: &mut Session, node: &LayoutNode, area: Rect) {
    match node {
        LayoutNode::Leaf(idx) => {
            if let Some(w) = session.app.widgets.get(*idx) {
                session.areas.insert(w.id.clone(), area);
            }
        }
        LayoutNode::Split {
            dir,
            ratio,
            a,
            b,
            handle_id,
        } => {
            let ratio = session
                .splitter_ratio
                .get(handle_id)
                .copied()
                .unwrap_or(*ratio)
                .clamp(50, 950);
            session
                .splitter_ratio
                .entry(handle_id.clone())
                .or_insert(ratio);
            let (left, handle, right) = split_axis(area, *dir, ratio);
            assign_node(session, a, left);
            if let Some(handle) = handle {
                session.splitter_handles.push(SplitterHandle {
                    id: handle_id.clone(),
                    area: handle,
                    direction: *dir,
                    split_area: area,
                });
            }
            assign_node(session, b, right);
        }
    }
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
        assert!(first >= 1 && first < 4);
        let (first, handle) = split_lengths(20, 500);
        assert!(handle);
        assert!(first >= 3 && first <= 16);
    }
}
