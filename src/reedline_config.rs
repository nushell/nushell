use crossterm::event::{KeyCode, KeyModifiers};
use nu_color_config::lookup_ansi_color_style;
use nu_protocol::{Config, EventType, ParsedKeybinding, ShellError};
use reedline::{
    default_emacs_keybindings, ContextMenuInput, EditCommand, Keybindings, ReedlineEvent,
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

pub(crate) fn create_keybindings(
    parsed_keybindings: &[ParsedKeybinding],
) -> Result<Keybindings, ShellError> {
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

    for keybinding in parsed_keybindings {
        let modifier = match keybinding.modifier.as_str() {
            "CONTROL" => KeyModifiers::CONTROL,
            "SHIFT" => KeyModifiers::CONTROL,
            _ => unimplemented!(),
        };

        let keycode = match keybinding.keycode.as_str() {
            c if c.starts_with("Char_") => {
                let char = c.replace("Char_", "");
                let char = char.chars().next().expect("correct");
                KeyCode::Char(char)
            }
            "down" => KeyCode::Down,
            _ => unimplemented!(),
        };

        let event = match &keybinding.event {
            EventType::Single(name) => match name.as_str() {
                "Complete" => ReedlineEvent::Complete,
                "ContextMenu" => ReedlineEvent::ContextMenu,
                _ => unimplemented!(),
            },
        };

        keybindings.add_binding(modifier, keycode, event);
    }

    Ok(keybindings)
}
