//! `tui split`: children side by side or stacked, with draggable handles.
use crate::keys::KeyPress;
use crate::session::Session;
use crate::widget::{Caps, Effect, Size, SplitState, TuiWidget, WidgetState};
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use serde::{Deserialize, Serialize};

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

    /// Set the size of child `index` so its edge lands `len` cells into a
    /// split `total` cells long. Sizes become percentages so the other
    /// children keep sharing the rest.
    pub fn place_handle(state: &mut SplitState, index: usize, len: u16, total: u16) {
        if total == 0 || index >= state.sizes.len() {
            return;
        }
        let percent = (len as u32 * 100 / total as u32).clamp(5, 95) as u16;
        state.sizes[index] = Size::Percent(percent);
    }

    /// Keyboard resize of the first child by one percent.
    fn nudge(&self, id: &str, state: &mut SplitState, grow: bool, session: &Session) {
        let Some(widget) = session.widget(id) else {
            return;
        };
        let Some(first) = widget.children.first() else {
            return;
        };
        let (Some(area), Some(child)) = (session.areas.get(id), session.areas.get(&first.id))
        else {
            return;
        };
        let (total, len) = match self.direction {
            SplitDir::Horizontal => (area.width, child.width),
            SplitDir::Vertical => (area.height, child.height),
        };
        if total == 0 {
            return;
        }
        let current = (len as u32 * 100 / total as u32) as u16;
        let next = if grow {
            (current + 1).min(95)
        } else {
            current.saturating_sub(1).max(5)
        };
        if let Some(size) = state.sizes.first_mut() {
            *size = Size::Percent(next);
        }
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
}
