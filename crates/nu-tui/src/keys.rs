//! Key event parsing and scripted-key replay for headless tests.
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use nu_protocol::{ShellError, Span, shell_error::generic::GenericError};

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

pub fn normalize_bind(s: &str) -> String {
    s.trim().to_ascii_lowercase().replace(' ', "")
}

/// Parse a comma-separated script of keys/mouse events.
///
/// Tokens: `enter`, `esc`, `tab`, `shift+tab`, `up`, `down`, `left`, `right`,
/// `home`, `end`, `pageup`, `pagedown`, `backspace`, `delete`, `space`,
/// `ctrl+c`, `ctrl+r`, `alt+a`, `f1`, a single character, `type:hello`,
/// `click:COL,ROW`, `scroll-up`, `scroll-down`, `drag:COL,ROW`.
pub fn parse_scripted_keys(script: &str, span: Span) -> Result<Vec<Event>, ShellError> {
    let mut events = Vec::new();
    for token in split_script(script) {
        events.extend(parse_token(&token, span)?);
    }
    Ok(events)
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
}
