//! Key chords: normalizing crossterm events, parsing `--bind`/`tui bind`
//! keys, and the scripted-key replay used by `tui debug`.
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use nu_protocol::{ShellError, Span, Value, shell_error::generic::GenericError};

/// A key press as widgets see it: the raw event plus its normalized chord
/// (`ctrl+r`, `shift+tab`, `a`).
#[derive(Debug, Clone)]
pub struct KeyPress {
    pub chord: String,
    pub event: KeyEvent,
}

impl KeyPress {
    pub fn new(event: KeyEvent) -> Self {
        Self {
            chord: key_event_to_string(event),
            event,
        }
    }

    /// Whether any of ctrl/alt/super is held. Plain-letter binds are
    /// suppressed while typing; modified ones still fire.
    pub fn has_modifier(&self) -> bool {
        self.event
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
    }
}

/// Normalize a key event into a stable string like `ctrl+r` or `shift+tab`.
pub fn key_event_to_string(key: KeyEvent) -> String {
    let mut parts: Vec<String> = Vec::new();

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl".into());
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt".into());
    }
    if key.modifiers.contains(KeyModifiers::SUPER) {
        parts.push("super".into());
    }

    let mut shift = key.modifiers.contains(KeyModifiers::SHIFT);

    let key_str = match key.code {
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::PageUp => "pageup".to_string(),
        KeyCode::PageDown => "pagedown".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::BackTab => {
            shift = true;
            "tab".to_string()
        }
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Insert => "insert".to_string(),
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Char(' ') => "space".to_string(),
        KeyCode::Char(c) => {
            let lower = c.to_ascii_lowercase();
            if c.is_uppercase() {
                shift = true;
            }
            lower.to_string()
        }
        KeyCode::F(n) => format!("f{n}"),
        _ => "unknown".to_string(),
    };

    if shift && key_str != "unknown" {
        parts.push("shift".into());
    }
    parts.push(key_str);
    parts.join("+")
}

/// A bind string in the form [`key_event_to_string`] produces, so
/// `shift+ctrl+s` and `Ctrl+Shift+S` both match the chord `ctrl+shift+s`.
/// Text that is not a chord is only lowercased and stripped of spaces.
pub fn normalize_bind(s: &str) -> String {
    let flat = s.trim().to_ascii_lowercase().replace(' ', "");
    match parse_key_token(&flat, Span::unknown()) {
        Ok(event) => key_event_to_string(event),
        Err(_) => flat,
    }
}

/// A chord from a bind value: a string like `ctrl+s`, or a reedline-style
/// record `{modifier: control, keycode: char_s}` as used in
/// `$env.config.keybindings`.
pub fn chord_from_value(value: &Value) -> Result<String, ShellError> {
    match value {
        Value::String { val, .. } => Ok(normalize_bind(val)),
        Value::Record { val, .. } => {
            let modifier = val
                .get("modifier")
                .and_then(|v| v.as_str().ok())
                .unwrap_or("none");
            let keycode = val
                .get("keycode")
                .and_then(|v| v.as_str().ok())
                .ok_or_else(|| ShellError::MissingParameter {
                    param_name: "keycode".into(),
                    span: value.span(),
                })?;
            Ok(normalize_binding(modifier, keycode))
        }
        other => Err(ShellError::TypeMismatch {
            err_message: format!(
                "expected a key string like 'ctrl+s' or a {{modifier, keycode}} record, found {}",
                other.get_type()
            ),
            span: other.span(),
        }),
    }
}

/// Map reedline config keys (`control` + `char_r`) onto
/// [`key_event_to_string`] chords (`ctrl+r`).
pub fn normalize_binding(modifier: &str, keycode: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let modifier = modifier.to_ascii_lowercase().replace('-', "_");
    if modifier != "none" && !modifier.is_empty() {
        if modifier.contains("ctrl") || modifier.contains("control") {
            parts.push("ctrl".into());
        }
        if modifier.contains("alt") {
            parts.push("alt".into());
        }
        if modifier.contains("super") || modifier.contains("meta") {
            parts.push("super".into());
        }
        if modifier.contains("shift") {
            parts.push("shift".into());
        }
    }

    let key = keycode.to_ascii_lowercase();
    let key = key.strip_prefix("char_").map(str::to_string).unwrap_or(key);
    if !key.is_empty() {
        parts.push(key);
    }
    parts.join("+")
}

/// Parse a comma-separated script of keys/mouse events.
///
/// Tokens: `enter`, `esc`, `tab`, `shift+tab`, `up`, `down`, `left`, `right`,
/// `home`, `end`, `pageup`, `pagedown`, `backspace`, `delete`, `space`,
/// `ctrl+c`, `ctrl+r`, `alt+a`, `f1`, a single character, `type:hello`,
/// `click:COL,ROW`, `scroll-up`, `scroll-down`, `drag:COL,ROW`.
pub fn parse_scripted_keys(script: &str, span: Span) -> Result<Vec<Event>, ShellError> {
    parse_key_tokens(split_script(script), span)
}

/// Parse tokens given one per list element (`--keys [down enter "click:3,4"]`).
pub fn parse_key_tokens(
    tokens: impl IntoIterator<Item = String>,
    span: Span,
) -> Result<Vec<Event>, ShellError> {
    let mut events = Vec::new();
    for token in tokens {
        events.extend(parse_token(token.trim(), span)?);
    }
    Ok(events)
}

/// `--keys` as either a comma-separated string or a list of tokens.
pub fn key_script_from_value(value: &Value, span: Span) -> Result<Vec<Event>, ShellError> {
    match value {
        Value::String { val, .. } => parse_scripted_keys(val, span),
        Value::List { vals, .. } => {
            let mut tokens = Vec::with_capacity(vals.len());
            for v in vals {
                tokens.push(v.as_str()?.to_string());
            }
            parse_key_tokens(tokens, span)
        }
        other => Err(ShellError::TypeMismatch {
            err_message: format!(
                "expected a key script string or list, found {}",
                other.get_type()
            ),
            span: other.span(),
        }),
    }
}

fn split_script(script: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut rest = script.trim();
    while !rest.is_empty() {
        rest = rest.trim_start_matches([',', ' ', '\t']);
        if rest.is_empty() {
            break;
        }
        let lower = rest.to_ascii_lowercase();
        let (token, next) = if lower.starts_with("click:") || lower.starts_with("drag:") {
            take_xy_token(rest)
        } else if let Some(idx) = rest.find(',') {
            (&rest[..idx], rest[idx + 1..].trim_start())
        } else {
            (rest, "")
        };
        let token = token.trim();
        if !token.is_empty() {
            tokens.push(token.to_string());
        }
        rest = next;
    }
    tokens
}

/// `click:10,5` contains a comma; take `prefix:X,Y` as one token.
fn take_xy_token(rest: &str) -> (&str, &str) {
    let Some(colon) = rest.find(':') else {
        return (rest, "");
    };
    let after = &rest[colon + 1..];
    let Some(comma) = after.find(',') else {
        return (rest, "");
    };
    let coords = &after[comma + 1..];
    let end = coords
        .find(|c: char| c == ',' || c.is_whitespace())
        .map(|i| colon + 1 + comma + 1 + i)
        .unwrap_or(rest.len());
    let token = rest[..end].trim_end();
    let next = rest[end..].trim_start_matches([',', ' ', '\t']);
    (token, next)
}

fn parse_token(token: &str, span: Span) -> Result<Vec<Event>, ShellError> {
    let lower = token.to_ascii_lowercase();

    if let Some(rest) = lower.strip_prefix("type:") {
        let original = token.get(5..).unwrap_or(rest);
        return Ok(original
            .chars()
            .map(|c| Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)))
            .collect());
    }

    if let Some(rest) = lower.strip_prefix("click:") {
        let (column, row) = parse_xy(rest, span)?;
        return Ok(vec![Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })]);
    }

    if let Some(rest) = lower.strip_prefix("drag:") {
        let (column, row) = parse_xy(rest, span)?;
        return Ok(vec![Event::Mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })]);
    }

    if lower == "scroll-up" {
        return Ok(vec![Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })]);
    }
    if lower == "scroll-down" {
        return Ok(vec![Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })]);
    }

    Ok(vec![Event::Key(parse_key_token(&lower, span)?)])
}

fn parse_xy(s: &str, span: Span) -> Result<(u16, u16), ShellError> {
    let mut parts = s.split(',');
    let x = parts.next();
    let y = parts.next();
    match (x, y) {
        (Some(x), Some(y)) => {
            let column = x.trim().parse::<u16>().map_err(|_| script_error(span, s))?;
            let row = y.trim().parse::<u16>().map_err(|_| script_error(span, s))?;
            Ok((column, row))
        }
        _ => Err(script_error(span, s)),
    }
}

fn parse_key_token(token: &str, span: Span) -> Result<KeyEvent, ShellError> {
    let mut mods = KeyModifiers::NONE;
    let mut key_part = token;

    loop {
        if let Some(rest) = key_part.strip_prefix("ctrl+") {
            mods |= KeyModifiers::CONTROL;
            key_part = rest;
            continue;
        }
        if let Some(rest) = key_part.strip_prefix("alt+") {
            mods |= KeyModifiers::ALT;
            key_part = rest;
            continue;
        }
        if let Some(rest) = key_part.strip_prefix("shift+") {
            mods |= KeyModifiers::SHIFT;
            key_part = rest;
            continue;
        }
        if let Some(rest) = key_part.strip_prefix("super+") {
            mods |= KeyModifiers::SUPER;
            key_part = rest;
            continue;
        }
        break;
    }

    let code = match key_part {
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" if mods.contains(KeyModifiers::SHIFT) => {
            mods.remove(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "backspace" | "bs" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "space" => KeyCode::Char(' '),
        // `f1`..`f12`; a bare `f` is the letter.
        other if other.len() > 1 && other.len() <= 3 && other.starts_with('f') => {
            let n = other[1..]
                .parse::<u8>()
                .map_err(|_| script_error(span, token))?;
            KeyCode::F(n)
        }
        other if other.chars().count() == 1 => {
            let Some(c) = other.chars().next() else {
                return Err(script_error(span, token));
            };
            if mods.contains(KeyModifiers::SHIFT) {
                KeyCode::Char(c.to_ascii_uppercase())
            } else {
                KeyCode::Char(c)
            }
        }
        _ => return Err(script_error(span, token)),
    };

    Ok(KeyEvent::new(code, mods))
}

fn script_error(span: Span, token: &str) -> ShellError {
    ShellError::Generic(GenericError::new(
        "invalid tui key script",
        format!("cannot parse key token '{token}'"),
        span,
    ))
}

/// Editing keys shared by text boxes and search boxes. Returns `true` when
/// the key changed the text or cursor.
pub fn apply_edit(key: KeyEvent, text: &mut String, cursor: &mut usize) -> bool {
    let len = text.chars().count();
    *cursor = (*cursor).min(len);
    match (key.code, key.modifiers) {
        (KeyCode::Char(c), m)
            if !m.contains(KeyModifiers::CONTROL) && !m.contains(KeyModifiers::ALT) =>
        {
            let byte = byte_index(text, *cursor);
            text.insert(byte, c);
            *cursor += 1;
            true
        }
        (KeyCode::Backspace, _) => {
            if *cursor > 0 {
                remove_char(text, *cursor - 1);
                *cursor -= 1;
            }
            true
        }
        (KeyCode::Delete, _) => {
            if *cursor < text.chars().count() {
                remove_char(text, *cursor);
            }
            true
        }
        (KeyCode::Left, _) => {
            *cursor = cursor.saturating_sub(1);
            true
        }
        (KeyCode::Right, _) => {
            *cursor = (*cursor + 1).min(text.chars().count());
            true
        }
        (KeyCode::Home, _) => {
            *cursor = 0;
            true
        }
        (KeyCode::Char('a'), m) if m.contains(KeyModifiers::CONTROL) => {
            *cursor = 0;
            true
        }
        (KeyCode::End, _) => {
            *cursor = text.chars().count();
            true
        }
        (KeyCode::Char('e'), m) if m.contains(KeyModifiers::CONTROL) => {
            *cursor = text.chars().count();
            true
        }
        (KeyCode::Char('u'), m) if m.contains(KeyModifiers::CONTROL) => {
            text.clear();
            *cursor = 0;
            true
        }
        _ => false,
    }
}

fn remove_char(text: &mut String, char_idx: usize) {
    let start = byte_index(text, char_idx);
    let end = byte_index(text, char_idx + 1);
    text.replace_range(start..end, "");
}

fn byte_index(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(text.len())
}

/// The chord's only character, for mnemonic matching.
pub fn single_char(chord: &str) -> Option<char> {
    let mut chars = chord.chars();
    let c = chars.next()?;
    chars.next().is_none().then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_without_modifiers_is_plain() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(key_event_to_string(key), "q");
    }

    #[test]
    fn ctrl_r_normalizes() {
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_string(key), "ctrl+r");
    }

    #[test]
    fn script_type_expands_chars() {
        let events = parse_scripted_keys("type:ab", Span::test_data()).expect("parse");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn click_token_keeps_comma() {
        let events = parse_scripted_keys("click:10,5,enter", Span::test_data()).expect("parse");
        assert_eq!(events.len(), 2);
        match &events[0] {
            Event::Mouse(m) => {
                assert_eq!(m.column, 10);
                assert_eq!(m.row, 5);
            }
            other => panic!("expected mouse, got {other:?}"),
        }
    }

    #[test]
    fn list_tokens_parse_one_each() {
        let value = Value::test_list(vec![
            Value::test_string("down"),
            Value::test_string("click:3,4"),
            Value::test_string("type:hi"),
        ]);
        let events = key_script_from_value(&value, Span::test_data()).expect("parse");
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn reedline_record_maps_to_chord() {
        let mut rec = nu_protocol::Record::new();
        rec.insert("modifier", Value::test_string("control"));
        rec.insert("keycode", Value::test_string("char_s"));
        let chord = chord_from_value(&Value::test_record(rec)).expect("chord");
        assert_eq!(chord, "ctrl+s");
    }

    #[test]
    fn normalize_control_char_r() {
        assert_eq!(normalize_binding("control", "char_r"), "ctrl+r");
        assert_eq!(normalize_binding("none", "tab"), "tab");
    }

    #[test]
    fn bind_strings_canonicalize_modifier_order() {
        assert_eq!(normalize_bind("shift+ctrl+s"), "ctrl+shift+s");
        assert_eq!(normalize_bind("Alt + Ctrl + f"), "ctrl+alt+f");
        assert_eq!(normalize_bind("shift+tab"), "shift+tab");
        assert_eq!(normalize_bind("/"), "/");
        let event = KeyEvent::new(
            KeyCode::Char('S'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!(key_event_to_string(event), normalize_bind("shift+ctrl+s"));
    }
}
