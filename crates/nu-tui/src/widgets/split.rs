//! `tui split`: children side by side or stacked, with draggable handles.
use crate::keys::KeyPress;
use crate::layout::pin_fills;
use crate::session::Session;
use crate::widget::{Caps, Effect, Size, SplitState, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SplitDir {
    /// Side by side (left | right).
    Horizontal,
    /// Stacked (top / bottom).
    Vertical,
}

impl SplitDir {
    pub fn as_str(self) -> &'static str {
        match self {
            SplitDir::Horizontal => "horizontal",
            SplitDir::Vertical => "vertical",
        }
    }

    /// Children of these sizes laid out in `area`, and the spacers around
    /// them, with a one-cell handle between each pair when the split is
    /// `resizable`. Drawing and resizing both use it, so a resize is judged
    /// by the cells the new size will really get.
    ///
    /// Many `Fill` children reach the solver pinned to their shares, see
    /// [`pin_fills`].
    pub fn split(self, sizes: &[Size], resizable: bool, area: Rect) -> (Rc<[Rect]>, Rc<[Rect]>) {
        let constraints = sizes.iter().map(|s| s.to_constraint()).collect();
        let constraints = pin_fills(constraints, u16::from(resizable), self.length(area));
        self.layout(constraints, resizable).split_with_spacers(area)
    }

    /// The ratatui layout for `constraints` along this direction.
    fn layout(self, constraints: impl IntoIterator<Item = Constraint>, resizable: bool) -> Layout {
        let direction = match self {
            SplitDir::Horizontal => Direction::Horizontal,
            SplitDir::Vertical => Direction::Vertical,
        };
        Layout::default()
            .direction(direction)
            .constraints(constraints)
            .spacing(u16::from(resizable))
    }

    /// The cells `rect` covers along this direction.
    fn length(self, rect: Rect) -> u16 {
        match self {
            SplitDir::Horizontal => rect.width,
            SplitDir::Vertical => rect.height,
        }
    }

    /// Where `rect` ends along this direction: the column after it, or
    /// the row below it for `--vertical`.
    fn end(self, rect: Rect) -> u16 {
        match self {
            SplitDir::Horizontal => rect.right(),
            SplitDir::Vertical => rect.bottom(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitWidget {
    pub direction: SplitDir,
    /// One size per child. Missing entries are `1fr`.
    pub sizes: Vec<Size>,
}

impl SplitWidget {
    /// Sizes padded to `n` children.
    pub fn sizes_for(&self, n: usize) -> Vec<Size> {
        let mut sizes = self.sizes.clone();
        sizes.truncate(n);
        sizes.resize(n, Size::Fill(1));
        sizes
    }

    /// Mouse drag: move handle `index` of a split drawn in `area` as close
    /// as it can get to `at`, the pointer's column (row for `--vertical`).
    /// Sizes become percentages so the other children keep sharing the
    /// rest.
    pub fn place_handle(
        state: &mut SplitState,
        direction: SplitDir,
        index: usize,
        area: Rect,
        at: u16,
    ) {
        move_edge(state, direction, index, area, |edge| {
            Some(edge.abs_diff(at))
        });
    }

    /// Keyboard resize of the first child: move its edge to the nearest
    /// cell a percentage can reach in that direction. On splits up to 100
    /// cells that is the next cell; on wider ones a percentage covers more
    /// than a cell, so a press can skip one.
    fn nudge(&self, id: &str, state: &mut SplitState, grow: bool, session: &Session) {
        // Only a split the mouse can resize: one of fixed-height leaves
        // (a button row) gets no handle and must keep its sizes.
        if !session.handles.iter().any(|h| h.id == id && h.index == 0) {
            return;
        }
        let Some(&area) = session.areas.get(id) else {
            return;
        };
        // Measure from the sizes rather than the last drawn frame: several
        // presses can arrive before the next one.
        let (segments, _) = self.direction.split(&state.sizes, true, area);
        let Some(current) = segments.first().map(|&rect| self.direction.end(rect)) else {
            return;
        };
        move_edge(state, self.direction, 0, area, |edge| {
            let ahead = if grow { edge > current } else { edge < current };
            ahead.then(|| edge.abs_diff(current))
        });
    }
}

/// Percentages a dragged or nudged child can take. Past these the handle
/// is within a twentieth of the split's edge.
const MIN_PERCENT: u16 = 5;
/// See [`MIN_PERCENT`].
const MAX_PERCENT: u16 = 95;

/// Give child `index` of a split drawn in `area` the percentage whose far
/// edge has the lowest `cost` (`None` rules an edge out). Every candidate
/// is laid out for real, so the choice matches what gets drawn whatever
/// ratatui's rounding does and however the other children are sized.
/// Candidates that would leave any child with no cells are skipped. When
/// several percentages end on the same cell the middle one is kept: it is
/// closest to the exact share, so the pane keeps its proportion when the
/// terminal is resized.
fn move_edge(
    state: &mut SplitState,
    direction: SplitDir,
    index: usize,
    area: Rect,
    cost: impl Fn(u16) -> Option<u16>,
) {
    if index >= state.sizes.len() {
        return;
    }
    let mut trial = state.sizes.clone();
    let reachable: Vec<(u16, u16)> = (MIN_PERCENT..=MAX_PERCENT)
        .filter_map(|percent| {
            trial[index] = Size::Percent(percent);
            let (segments, _) = direction.split(&trial, true, area);
            let edge = direction.end(*segments.get(index)?);
            segments
                .iter()
                .all(|rect| !rect.is_empty())
                .then_some((percent, edge))
        })
        .collect();
    let Some((_, best)) = reachable
        .iter()
        .filter_map(|&(_, edge)| Some((cost(edge)?, edge)))
        .min()
    else {
        return;
    };
    let band: Vec<u16> = reachable
        .iter()
        .filter(|&&(_, edge)| edge == best)
        .map(|&(percent, _)| percent)
        .collect();
    if let Some(&percent) = band.get(band.len() / 2) {
        state.sizes[index] = Size::Percent(percent);
    }
}

impl TuiWidget for SplitWidget {
    fn type_name(&self) -> &'static str {
        "split"
    }

    fn caps(&self) -> Caps {
        Caps {
            focusable: true,
            container: true,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Min(5)
    }

    fn init_state(&self) -> WidgetState {
        WidgetState::Split(SplitState {
            sizes: self.sizes.clone(),
        })
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("direction", Value::string(self.direction.as_str(), span));
        rec.insert(
            "sizes",
            Value::list(self.sizes.iter().map(|s| s.to_value(span)).collect(), span),
        );
    }

    fn handle_key(
        &self,
        id: &str,
        state: &mut WidgetState,
        key: &KeyPress,
        session: &Session,
    ) -> Option<Vec<Effect>> {
        let grow = match (self.direction, key.chord.as_str()) {
            (SplitDir::Horizontal, "left" | "h") | (SplitDir::Vertical, "up" | "k") => false,
            (SplitDir::Horizontal, "right" | "l") | (SplitDir::Vertical, "down" | "j") => true,
            _ => return None,
        };
        let split = state.as_split_mut()?;
        self.nudge(id, split, grow, session);
        Some(Vec::new())
    }

    /// Containers draw nothing; the session draws the handles.
    fn render(
        &self,
        _id: &str,
        _state: &WidgetState,
        _frame: &mut Frame,
        _area: Rect,
        _session: &Session,
        _focused: bool,
    ) {
    }

    fn value(&self, _id: &str, state: &WidgetState, _session: &Session) -> Value {
        let span = Span::unknown();
        let sizes = state
            .as_split()
            .map(|s| s.sizes.clone())
            .unwrap_or_default();
        Value::list(sizes.iter().map(|s| s.to_value(span)).collect(), span)
    }
}

/// Parse `--sizes` entries: an int is cells, `"30%"`, `"1fr"`, `"min:10"`,
/// `"max:40"`.
pub fn parse_size(value: &Value) -> Result<Size, nu_protocol::ShellError> {
    let bad = |what: &str| nu_protocol::ShellError::TypeMismatch {
        err_message: format!(
            "expected a size like 12, \"30%\", \"1fr\", \"min:10\" or \"max:40\", found {what}"
        ),
        span: value.span(),
    };
    match value {
        Value::Int { val, .. } => Ok(Size::Length((*val).clamp(0, u16::MAX as i64) as u16)),
        Value::String { val, .. } => {
            let s = val.trim().to_ascii_lowercase();
            let num = |t: &str| t.trim().parse::<u16>().map_err(|_| bad(val));
            if let Some(p) = s.strip_suffix('%') {
                Ok(Size::Percent(num(p)?.min(100)))
            } else if let Some(f) = s.strip_suffix("fr") {
                Ok(Size::Fill(if f.is_empty() { 1 } else { num(f)?.max(1) }))
            } else if let Some(m) = s.strip_prefix("min:") {
                Ok(Size::Min(num(m)?))
            } else if let Some(m) = s.strip_prefix("max:") {
                Ok(Size::Max(num(m)?))
            } else {
                Ok(Size::Length(num(&s)?))
            }
        }
        other => Err(bad(&other.get_type().to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_parse_every_form() {
        assert_eq!(
            parse_size(&Value::test_int(12)).ok(),
            Some(Size::Length(12))
        );
        assert_eq!(
            parse_size(&Value::test_string("30%")).ok(),
            Some(Size::Percent(30))
        );
        assert_eq!(
            parse_size(&Value::test_string("2fr")).ok(),
            Some(Size::Fill(2))
        );
        assert_eq!(
            parse_size(&Value::test_string("fr")).ok(),
            Some(Size::Fill(1))
        );
        assert_eq!(
            parse_size(&Value::test_string("min:10")).ok(),
            Some(Size::Min(10))
        );
        assert!(parse_size(&Value::test_string("wide")).is_err());
    }

    #[test]
    fn many_fill_children_share_the_split_by_weight() {
        // Past the limit in `pin_fills`: the 39 `Fill` children share 341 cells
        // (400, less 20, less 39 handles) by weight, 2 of 40 for the `2fr`
        // child.
        let mut sizes = vec![Size::Length(20), Size::Fill(2)];
        sizes.resize(40, Size::Fill(1));
        let area = Rect::new(0, 0, 400, 10);
        let (segments, _) = SplitDir::Horizontal.split(&sizes, true, area);
        let widths: Vec<u16> = segments.iter().map(|rect| rect.width).collect();
        assert_eq!(widths[..2], [20, 17]);
        assert!(
            widths[2..].iter().all(|w| (8..=9).contains(w)),
            "{widths:?}"
        );
        assert_eq!(widths.iter().sum::<u16>(), 400 - 39);
        assert_eq!(segments.last().map(|rect| rect.right()), Some(400));
    }
}
