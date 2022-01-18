use crossterm::event::{KeyCode, KeyModifiers};
use nu_color_config::lookup_ansi_color_style;
use nu_protocol::{Config, EventType, ParsedKeybinding, ShellError};
use reedline::{
    default_emacs_keybindings, default_vi_insert_keybindings, default_vi_normal_keybindings,
    ContextMenuInput, EditCommand, Keybindings, ReedlineEvent,
};

// This creates an input object for the context menu based on the dictionary
// stored in the config variable
pub(crate) fn create_menu_input(config: &Config) -> ContextMenuInput {
    let mut input = ContextMenuInput::default();

    input = match config
        .menu_config
        .get("columns")
        .and_then(|value| value.as_integer().ok())
    {
        Some(value) => input.with_columns(value as u16),
        None => input,
    };

    input = input.with_col_width(
        config
            .menu_config
            .get("col_width")
            .and_then(|value| value.as_integer().ok())
            .map(|value| value as usize),
    );

    input = match config
        .menu_config
        .get("col_padding")
        .and_then(|value| value.as_integer().ok())
    {
        Some(value) => input.with_col_padding(value as usize),
        None => input,
    };

    input = match config
        .menu_config
        .get("text_style")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => input.with_text_style(lookup_ansi_color_style(&value)),
        None => input,
    };

    input = match config
        .menu_config
        .get("selected_text_style")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => input.with_selected_text_style(lookup_ansi_color_style(&value)),
        None => input,
    };

    input
}

pub enum KeybindingsMode {
    Emacs(Keybindings),
    Vi {
        insert_keybindings: Keybindings,
        normal_keybindings: Keybindings,
    },
}

pub(crate) fn create_keybindings(config: &Config) -> Result<KeybindingsMode, ShellError> {
    let parsed_keybindings = &config.keybindings;
    match config.edit_mode.as_str() {
        "emacs" => {
            let mut keybindings = default_emacs_keybindings();
            // temporal keybinding with multiple events
            keybindings.add_binding(
                KeyModifiers::SHIFT,
                KeyCode::BackTab,
                ReedlineEvent::Multiple(vec![
                    ReedlineEvent::Edit(vec![EditCommand::InsertChar('p')]),
                    ReedlineEvent::Enter,
                ]),
            );

            for parsed_keybinding in parsed_keybindings {
                if parsed_keybinding.mode.item.as_str() == "emacs" {
                    add_keybinding(&mut keybindings, parsed_keybinding)?
                }
            }

            Ok(KeybindingsMode::Emacs(keybindings))
        }
        _ => {
            let mut insert_keybindings = default_vi_insert_keybindings();
            let mut normal_keybindings = default_vi_normal_keybindings();

            for parsed_keybinding in parsed_keybindings {
                if parsed_keybinding.mode.item.as_str() == "vi_insert" {
                    add_keybinding(&mut insert_keybindings, parsed_keybinding)?
                } else if parsed_keybinding.mode.item.as_str() == "vi_normal" {
                    add_keybinding(&mut normal_keybindings, parsed_keybinding)?
                }
            }

            Ok(KeybindingsMode::Vi {
                insert_keybindings,
                normal_keybindings,
            })
        }
    }
}

fn add_keybinding(
    keybindings: &mut Keybindings,
    parsed_keybinding: &ParsedKeybinding,
) -> Result<(), ShellError> {
    let modifier = match parsed_keybinding.modifier.item.as_str() {
        "CONTROL" => KeyModifiers::CONTROL,
        "SHIFT" => KeyModifiers::SHIFT,
        "ALT" => KeyModifiers::ALT,
        "NONE" => KeyModifiers::NONE,
        "CONTROL | ALT" => KeyModifiers::CONTROL | KeyModifiers::ALT,
        _ => {
            return Err(ShellError::UnsupportedConfigValue(
                "CONTROL, SHIFT, ALT or NONE".to_string(),
                parsed_keybinding.modifier.item.clone(),
                parsed_keybinding.modifier.span,
            ))
        }
    };

    let keycode = match parsed_keybinding.keycode.item.as_str() {
        c if c.starts_with("Char_") => {
            let char = c.replace("Char_", "");
            let char = char.chars().next().expect("correct");
            KeyCode::Char(char)
        }
        "down" => KeyCode::Down,
        "up" => KeyCode::Up,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "Tab" => KeyCode::Tab,
        "BackTab" => KeyCode::BackTab,
        _ => {
            return Err(ShellError::UnsupportedConfigValue(
                "crossterm KeyCode".to_string(),
                parsed_keybinding.keycode.item.clone(),
                parsed_keybinding.keycode.span,
            ))
        }
    };

    let event = match &parsed_keybinding.event.item {
        EventType::Single(name) => match name.as_str() {
            "ActionHandler" => ReedlineEvent::ActionHandler,
            "Complete" => ReedlineEvent::Complete,
            "ContextMenu" => ReedlineEvent::ContextMenu,
            "NextElement" => ReedlineEvent::NextElement,
            "NextHistory" => ReedlineEvent::NextHistory,
            "PreviousElement" => ReedlineEvent::PreviousElement,
            "PreviousHistory" => ReedlineEvent::PreviousHistory,
            _ => {
                return Err(ShellError::UnsupportedConfigValue(
                    "crossterm EventType".to_string(),
                    name.clone(),
                    parsed_keybinding.event.span,
                ))
            }
        },
    };

    keybindings.add_binding(modifier, keycode, event);

    Ok(())
}
