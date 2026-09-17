//! `tui menu`: a bar with mnemonics and dropdowns.
use crate::keys::{KeyPress, single_char};
use crate::session::Session;
use crate::widget::{Caps, Effect, MenuState, TuiWidget, WidgetState};
use nu_protocol::engine::Closure;
use nu_protocol::{Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span as TSpan, Text};
use ratatui::widgets::{Clear, Paragraph};
use serde::{Deserialize, Serialize};

/// One entry on a menu bar or in a dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub label: String,
    /// Key that selects this item: `alt+<key>` on the bar, `<key>` inside an
    /// open dropdown. `&` before a letter in the label picks it; otherwise
    /// the first letter.
    pub mnemonic: Option<char>,
    /// Dropdown entries. Empty for a plain item.
    pub items: Vec<MenuItem>,
    /// Hook run when the item is activated. Without one, activating the
    /// item submits it.
    pub action: Option<Closure>,
}

impl MenuItem {
    /// Parse `&` mnemonics: `"&File"` → label `File`, mnemonic `f`.
    pub fn new(raw: &str, items: Vec<MenuItem>, action: Option<Closure>) -> Self {
        let mut label = String::with_capacity(raw.len());
        let mut mnemonic = None;
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '&' && mnemonic.is_none() {
                if let Some(next) = chars.next() {
                    mnemonic = Some(next.to_ascii_lowercase());
                    label.push(next);
                }
            } else {
                label.push(c);
            }
        }
        let mnemonic = mnemonic.or_else(|| {
            label
                .chars()
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
        });
        Self {
            label,
            mnemonic,
            items,
            action,
        }
    }

    /// Byte offset of the mnemonic letter in `label`, for underlining.
    pub fn mnemonic_index(&self) -> Option<usize> {
        let m = self.mnemonic?;
        self.label
            .char_indices()
            .find(|(_, c)| c.to_ascii_lowercase() == m)
            .map(|(i, _)| i)
    }

    pub fn to_value(&self, span: Span) -> Value {
        let mut rec = Record::new();
        rec.insert("name", Value::string(self.label.clone(), span));
        if let Some(m) = self.mnemonic {
            rec.insert("mnemonic", Value::string(m.to_string(), span));
        }
        if !self.items.is_empty() {
            rec.insert(
                "items",
                Value::list(self.items.iter().map(|i| i.to_value(span)).collect(), span),
            );
        }
        rec.insert("has_action", Value::bool(self.action.is_some(), span));
        Value::record(rec, span)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuWidget {
    pub items: Vec<MenuItem>,
}

impl MenuWidget {
    /// Index of the bar item whose mnemonic is `letter`.
    pub fn item_with_mnemonic(&self, letter: char) -> Option<usize> {
        let letter = letter.to_ascii_lowercase();
        self.items
            .iter()
            .position(|item| item.mnemonic == Some(letter))
    }

    /// Enter/Down/click on a bar item: open its dropdown, or activate it.
    pub fn open_or_activate(&self, state: &mut MenuState, session: &Session) -> Vec<Effect> {
        let has_items = self
            .items
            .get(state.selected)
            .is_some_and(|item| !item.items.is_empty());
        if has_items {
            state.open = true;
            state.item = 0;
            Vec::new()
        } else {
            self.activate(state, None, session)
        }
    }

    /// Run the item's hook, or submit it when it has none. A dropdown entry
    /// submits `{menu, item, row}` so the entry can act on the highlighted
    /// row; a bar item submits its name.
    fn activate(
        &self,
        state: &mut MenuState,
        sub: Option<usize>,
        session: &Session,
    ) -> Vec<Effect> {
        let Some(bar) = self.items.get(state.selected) else {
            return Vec::new();
        };
        let span = Span::unknown();
        let (item, selected) = match sub {
            Some(i) => {
                let Some(item) = bar.items.get(i) else {
                    return Vec::new();
                };
                let mut rec = Record::new();
                rec.insert("menu", Value::string(bar.label.clone(), span));
                rec.insert("item", Value::string(item.label.clone(), span));
                rec.insert("row", session.current_row());
                (item, Value::record(rec, span))
            }
            None => (bar, Value::string(bar.label.clone(), span)),
        };
        state.open = false;
        match &item.action {
            Some(closure) => vec![Effect::Hook(closure.clone())],
            None => vec![Effect::Submit(selected)],
        }
    }

    fn dropdown_key(&self, state: &mut MenuState, chord: &str, session: &Session) -> Vec<Effect> {
        let top_len = self.items.len();
        let sub_len = self
            .items
            .get(state.selected)
            .map(|i| i.items.len())
            .unwrap_or(0);
        match chord {
            "esc" => state.open = false,
            "tab" => {
                state.open = false;
                return vec![Effect::FocusNext];
            }
            "shift+tab" => {
                state.open = false;
                return vec![Effect::FocusPrev];
            }
            "up" | "k" => state.item = state.item.saturating_sub(1),
            "down" | "j" => state.item = (state.item + 1).min(sub_len.saturating_sub(1)),
            // Slide along the bar: the neighbour's dropdown opens if it has
            // one; otherwise the neighbour is selected and the dropdown closes.
            "left" | "h" | "right" | "l" => {
                state.selected = if matches!(chord, "left" | "h") {
                    state.selected.saturating_sub(1)
                } else {
                    (state.selected + 1).min(top_len.saturating_sub(1))
                };
                let has_items = self
                    .items
                    .get(state.selected)
                    .is_some_and(|i| !i.items.is_empty());
                state.item = 0;
                state.open = has_items;
            }
            "enter" => return self.activate(state, Some(state.item), session),
            _ => {
                if let Some(letter) = single_char(chord)
                    && let Some(idx) = self.items.get(state.selected).and_then(|bar| {
                        bar.items
                            .iter()
                            .position(|m| m.mnemonic == Some(letter.to_ascii_lowercase()))
                    })
                {
                    return self.activate(state, Some(idx), session);
                }
            }
        }
        Vec::new()
    }

    /// Horizontal offset and width of each bar item, as drawn.
    pub fn item_ranges(&self) -> Vec<(u16, u16)> {
        let mut x = 0u16;
        self.items
            .iter()
            .map(|item| {
                let width = item.label.chars().count() as u16 + 2;
                let range = (x, width);
                x = x.saturating_add(width);
                range
            })
            .collect()
    }

    /// Where the open dropdown is drawn, below its bar item.
    pub fn dropdown_rect(&self, state: &MenuState, bar: Rect) -> Option<Rect> {
        if !state.open {
            return None;
        }
        let entries = &self.items.get(state.selected)?.items;
        if entries.is_empty() {
            return None;
        }
        let (offset, _) = self.item_ranges().get(state.selected).copied()?;
        let width = entries
            .iter()
            .map(|i| i.label.chars().count())
            .max()
            .unwrap_or(0) as u16
            + 4;
        let max_x = bar.x.saturating_add(bar.width.saturating_sub(width));
        Some(Rect {
            x: bar.x.saturating_add(offset).min(max_x),
            y: bar.y.saturating_add(bar.height),
            width: width.min(bar.width),
            height: entries.len() as u16 + 2,
        })
    }

    /// A click while the dropdown is open: activate the entry under the
    /// mouse, or close the dropdown.
    pub fn click_dropdown(
        &self,
        state: &mut MenuState,
        rect: Rect,
        x: u16,
        y: u16,
        session: &Session,
    ) -> Vec<Effect> {
        if rect.contains(Position::new(x, y)) {
            let row = y.saturating_sub(rect.y).saturating_sub(1) as usize;
            return self.activate(state, Some(row), session);
        }
        state.open = false;
        Vec::new()
    }

    /// The open dropdown, drawn last so it sits on top of the page.
    pub fn render_dropdown(
        &self,
        state: &MenuState,
        rect: Rect,
        frame: &mut Frame,
        session: &Session,
    ) {
        let theme = &session.theme;
        let Some(entries) = self.items.get(state.selected).map(|i| &i.items) else {
            return;
        };
        let block = super::framed("", true, theme);
        let inner = block.inner(rect);
        frame.render_widget(Clear, rect);
        frame.render_widget(block, rect);
        let lines: Vec<Line> = entries
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == state.item {
                    theme.text().patch(theme.selected())
                } else {
                    theme.text()
                };
                let mut spans = vec![TSpan::styled(" ", style)];
                spans.extend(mnemonic_spans(item, style));
                spans.push(TSpan::styled(" ", style));
                Line::from(spans)
            })
            .collect();
        frame.render_widget(Paragraph::new(Text::from(lines)).style(theme.text()), inner);
    }
}

/// The label with its mnemonic letter underlined.
fn mnemonic_spans(item: &MenuItem, style: Style) -> Vec<TSpan<'static>> {
    let Some(idx) = item.mnemonic_index() else {
        return vec![TSpan::styled(item.label.clone(), style)];
    };
    let (before, rest) = item.label.split_at(idx);
    let mut chars = rest.chars();
    let letter = chars.next().map(|c| c.to_string()).unwrap_or_default();
    let after: String = chars.collect();
    vec![
        TSpan::styled(before.to_string(), style),
        TSpan::styled(letter, style.add_modifier(Modifier::UNDERLINED)),
        TSpan::styled(after, style),
    ]
}

impl TuiWidget for MenuWidget {
    fn type_name(&self) -> &'static str {
        "menu"
    }

    fn caps(&self) -> Caps {
        Caps {
            focusable: true,
            chrome: true,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Length(1)
    }

    fn init_state(&self) -> WidgetState {
        WidgetState::Menu(MenuState::default())
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert(
            "items",
            Value::list(self.items.iter().map(|i| i.to_value(span)).collect(), span),
        );
    }

    fn captures_keys(&self, state: &WidgetState) -> bool {
        state.as_menu().is_some_and(|m| m.open)
    }

    fn handle_key(
        &self,
        _id: &str,
        state: &mut WidgetState,
        key: &KeyPress,
        session: &Session,
    ) -> Option<Vec<Effect>> {
        let menu = state.as_menu_mut()?;
        if menu.open {
            return Some(self.dropdown_key(menu, &key.chord, session));
        }
        let len = self.items.len();
        if len == 0 {
            return None;
        }
        match key.chord.as_str() {
            "left" | "h" => menu.selected = menu.selected.saturating_sub(1),
            "right" | "l" => menu.selected = (menu.selected + 1).min(len - 1),
            "home" => menu.selected = 0,
            "end" => menu.selected = len - 1,
            "down" | "j" | "enter" => return Some(self.open_or_activate(menu, session)),
            _ => return None,
        }
        Some(Vec::new())
    }

    fn click(
        &self,
        id: &str,
        state: &mut WidgetState,
        area: Rect,
        x: u16,
        _y: u16,
        session: &Session,
    ) -> Vec<Effect> {
        let Some(menu) = state.as_menu_mut() else {
            return Vec::new();
        };
        let rel = x.saturating_sub(area.x);
        let mut effects = vec![Effect::Focus(id.to_string())];
        if let Some(idx) = self
            .item_ranges()
            .iter()
            .position(|(x, w)| rel >= *x && rel < x + w)
        {
            menu.selected = idx;
            effects.extend(self.open_or_activate(menu, session));
        }
        effects
    }

    fn render(
        &self,
        id: &str,
        state: &WidgetState,
        frame: &mut Frame,
        area: Rect,
        session: &Session,
        focused: bool,
    ) {
        let theme = &session.theme;
        let menu = state.as_menu().cloned().unwrap_or_default();
        let highlight = focused || menu.open;
        let mut spans = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            let style = if i == menu.selected && highlight {
                theme.text().patch(theme.selected())
            } else {
                theme.text()
            };
            spans.push(TSpan::styled(" ", style));
            spans.extend(mnemonic_spans(item, style));
            spans.push(TSpan::styled(" ", style));
        }
        let bg = if session.is_focused(id) {
            theme.border(true)
        } else {
            theme.text()
        };
        frame.render_widget(Paragraph::new(Line::from(spans)).style(bg), area);
    }

    fn value(&self, _id: &str, state: &WidgetState, _session: &Session) -> Value {
        let span = Span::unknown();
        let menu = state.as_menu().cloned().unwrap_or_default();
        match self.items.get(menu.selected) {
            Some(item) => Value::string(item.label.clone(), span),
            None => Value::nothing(span),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnemonic_marker_is_stripped_and_defaults_to_first_letter() {
        let item = MenuItem::new("&File", Vec::new(), None);
        assert_eq!(item.label, "File");
        assert_eq!(item.mnemonic, Some('f'));
        let item = MenuItem::new("E&xit", Vec::new(), None);
        assert_eq!(item.label, "Exit");
        assert_eq!(item.mnemonic, Some('x'));
        let item = MenuItem::new("View", Vec::new(), None);
        assert_eq!(item.mnemonic, Some('v'));
    }
}
