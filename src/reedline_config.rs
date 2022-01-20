use crossterm::event::{KeyCode, KeyModifiers};
use nu_color_config::lookup_ansi_color_style;
use nu_protocol::{extract_value, Config, ParsedKeybinding, ShellError, Span, Value};
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
                if parsed_keybinding.mode.into_string("", config).as_str() == "emacs" {
                    add_keybinding(&mut keybindings, parsed_keybinding, config)?
                }
            }

            Ok(KeybindingsMode::Emacs(keybindings))
        }
        _ => {
            let mut insert_keybindings = default_vi_insert_keybindings();
            let mut normal_keybindings = default_vi_normal_keybindings();

            for parsed_keybinding in parsed_keybindings {
                if parsed_keybinding.mode.into_string("", config).as_str() == "vi_insert" {
                    add_keybinding(&mut insert_keybindings, parsed_keybinding, config)?
                } else if parsed_keybinding.mode.into_string("", config).as_str() == "vi_normal" {
                    add_keybinding(&mut normal_keybindings, parsed_keybinding, config)?
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
    keybinding: &ParsedKeybinding,
    config: &Config,
) -> Result<(), ShellError> {
    let modifier = match keybinding
        .modifier
        .into_string("", config)
        .to_lowercase()
        .as_str()
    {
        "control" => KeyModifiers::CONTROL,
        "shift" => KeyModifiers::SHIFT,
        "alt" => KeyModifiers::ALT,
        "none" => KeyModifiers::NONE,
        "control | shift" => KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        "control | alt" => KeyModifiers::CONTROL | KeyModifiers::ALT,
        "control | alt | shift" => KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
        _ => {
            return Err(ShellError::UnsupportedConfigValue(
                keybinding.modifier.into_abbreviated_string(config),
                "CONTROL, SHIFT, ALT or NONE".to_string(),
                keybinding.modifier.span()?,
            ))
        }
    };

    let keycode = match keybinding
        .keycode
        .into_string("", config)
        .to_lowercase()
        .as_str()
    {
        "backspace" => KeyCode::Backspace,
        "enter" => KeyCode::Enter,
        c if c.starts_with("char_") => {
            let char = c.replace("char_", "");
            let char = char.chars().next().ok_or({
                ShellError::UnsupportedConfigValue(
                    c.to_string(),
                    "char_ plus char".to_string(),
                    keybinding.keycode.span()?,
                )
            })?;
            KeyCode::Char(char)
        }
        "down" => KeyCode::Down,
        "up" => KeyCode::Up,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        // TODO: Add KeyCode::F(u8) for function keys
        "null" => KeyCode::Null,
        "esc" | "escape" => KeyCode::Esc,
        _ => {
            return Err(ShellError::UnsupportedConfigValue(
                keybinding.keycode.into_abbreviated_string(config),
                "crossterm KeyCode".to_string(),
                keybinding.keycode.span()?,
            ))
        }
    };

    let event = parse_event(keybinding.event.clone(), config)?;

    keybindings.add_binding(modifier, keycode, event);

    Ok(())
}

fn parse_event(value: Value, config: &Config) -> Result<ReedlineEvent, ShellError> {
    match value {
        Value::Record { cols, vals, span } => {
            let event = match extract_value("send", &cols, &vals, &span) {
                Ok(event) => match event.into_string("", config).to_lowercase().as_str() {
                    "none" => ReedlineEvent::None,
                    "actionhandler" => ReedlineEvent::ActionHandler,
                    "clearscreen" => ReedlineEvent::ClearScreen,
                    "contextmenu" => ReedlineEvent::ContextMenu,
                    "complete" => ReedlineEvent::Complete,
                    "ctrld" => ReedlineEvent::CtrlD,
                    "ctrlc" => ReedlineEvent::CtrlC,
                    "enter" => ReedlineEvent::Enter,
                    "esc" | "escape" => ReedlineEvent::Esc,
                    "up" => ReedlineEvent::Up,
                    "down" => ReedlineEvent::Down,
                    "right" => ReedlineEvent::Right,
                    "left" => ReedlineEvent::Left,
                    "nextelement" => ReedlineEvent::NextElement,
                    "nexthistory" => ReedlineEvent::NextHistory,
                    "previouselement" => ReedlineEvent::PreviousElement,
                    "previoushistory" => ReedlineEvent::PreviousHistory,
                    "searchhistory" => ReedlineEvent::SearchHistory,
                    "repaint" => ReedlineEvent::Repaint,
                    // TODO: add ReedlineEvent::Mouse
                    // TODO: add ReedlineEvent::Resize
                    // TODO: add ReedlineEvent::Paste
                    "edit" => {
                        let edit = extract_value("edit", &cols, &vals, &span)?;
                        let edit = parse_edit(edit, config)?;

                        ReedlineEvent::Edit(vec![edit])
                    }
                    v => {
                        return Err(ShellError::UnsupportedConfigValue(
                            v.to_string(),
                            "Reedline event".to_string(),
                            span,
                        ))
                    }
                },
                Err(_) => {
                    let edit = extract_value("edit", &cols, &vals, &span);
                    let edit = match edit {
                        Ok(edit_value) => parse_edit(edit_value, config)?,
                        Err(_) => {
                            return Err(ShellError::MissingConfigValue(
                                "send or edit".to_string(),
                                span,
                            ))
                        }
                    };

                    ReedlineEvent::Edit(vec![edit])
                }
            };

            Ok(event)
        }
        Value::List { vals, .. } => {
            let events = vals
                .into_iter()
                .map(|value| parse_event(value, config))
                .collect::<Result<Vec<ReedlineEvent>, ShellError>>()?;

            Ok(ReedlineEvent::Multiple(events))
        }
        v => Err(ShellError::UnsupportedConfigValue(
            v.into_abbreviated_string(config),
            "record or list of records".to_string(),
            v.span()?,
        )),
    }
}

fn parse_edit(edit: &Value, config: &Config) -> Result<EditCommand, ShellError> {
    let edit = match edit {
        Value::Record {
            cols: edit_cols,
            vals: edit_vals,
            span: edit_span,
        } => {
            let cmd = extract_value("cmd", edit_cols, edit_vals, edit_span)?;

            match cmd.into_string("", config).to_lowercase().as_str() {
                "movetostart" => EditCommand::MoveToStart,
                "movetolinestart" => EditCommand::MoveToLineStart,
                "movetoend" => EditCommand::MoveToEnd,
                "movetolineend" => EditCommand::MoveToLineEnd,
                "moveleft" => EditCommand::MoveLeft,
                "moveright" => EditCommand::MoveRight,
                "movewordleft" => EditCommand::MoveWordLeft,
                "movewordright" => EditCommand::MoveWordRight,
                "insertchar" => {
                    let char = extract_char("value", edit_cols, edit_vals, config, edit_span)?;
                    EditCommand::InsertChar(char)
                }
                "insertstring" => {
                    let value = extract_value("value", edit_cols, edit_vals, edit_span)?;
                    EditCommand::InsertString(value.into_string("", config))
                }
                "backspace" => EditCommand::Backspace,
                "delete" => EditCommand::Delete,
                "backspaceword" => EditCommand::BackspaceWord,
                "deleteword" => EditCommand::DeleteWord,
                "clear" => EditCommand::Clear,
                "cleartolineend" => EditCommand::ClearToLineEnd,
                "cutcurrentline" => EditCommand::CutCurrentLine,
                "cutfromstart" => EditCommand::CutFromStart,
                "cutfromlinestart" => EditCommand::CutFromLineStart,
                "cuttoend" => EditCommand::CutToEnd,
                "cuttolineend" => EditCommand::CutToLineEnd,
                "cutwordleft" => EditCommand::CutWordLeft,
                "cutwordright" => EditCommand::CutWordRight,
                "pastecutbufferbefore" => EditCommand::PasteCutBufferBefore,
                "pastecutbufferafter" => EditCommand::PasteCutBufferAfter,
                "uppercaseword" => EditCommand::UppercaseWord,
                "lowercaseword" => EditCommand::LowercaseWord,
                "capitalizechar" => EditCommand::CapitalizeChar,
                "swapwords" => EditCommand::SwapWords,
                "swapgraphemes" => EditCommand::SwapGraphemes,
                "undo" => EditCommand::Undo,
                "redo" => EditCommand::Redo,
                "cutrightuntil" => {
                    let char = extract_char("value", edit_cols, edit_vals, config, edit_span)?;
                    EditCommand::CutRightUntil(char)
                }
                "cutrightbefore" => {
                    let char = extract_char("value", edit_cols, edit_vals, config, edit_span)?;
                    EditCommand::CutRightBefore(char)
                }
                "moverightuntil" => {
                    let char = extract_char("value", edit_cols, edit_vals, config, edit_span)?;
                    EditCommand::MoveRightUntil(char)
                }
                "moverightbefore" => {
                    let char = extract_char("value", edit_cols, edit_vals, config, edit_span)?;
                    EditCommand::MoveRightBefore(char)
                }
                "cutleftuntil" => {
                    let char = extract_char("value", edit_cols, edit_vals, config, edit_span)?;
                    EditCommand::CutLeftUntil(char)
                }
                "cutleftbefore" => {
                    let char = extract_char("value", edit_cols, edit_vals, config, edit_span)?;
                    EditCommand::CutLeftBefore(char)
                }
                "moveleftuntil" => {
                    let char = extract_char("value", edit_cols, edit_vals, config, edit_span)?;
                    EditCommand::MoveLeftUntil(char)
                }
                "moveleftbefore" => {
                    let char = extract_char("value", edit_cols, edit_vals, config, edit_span)?;
                    EditCommand::MoveLeftBefore(char)
                }
                e => {
                    return Err(ShellError::UnsupportedConfigValue(
                        e.to_string(),
                        "reedline EditCommand".to_string(),
                        edit.span()?,
                    ))
                }
            }
        }
        e => {
            return Err(ShellError::UnsupportedConfigValue(
                e.into_abbreviated_string(config),
                "record with EditCommand".to_string(),
                edit.span()?,
            ))
        }
    };

    Ok(edit)
}

fn extract_char<'record>(
    name: &str,
    cols: &'record [String],
    vals: &'record [Value],
    config: &Config,
    span: &Span,
) -> Result<char, ShellError> {
    let value = extract_value(name, cols, vals, span)?;

    value
        .into_string("", config)
        .chars()
        .next()
        .ok_or_else(|| ShellError::MissingConfigValue("char to insert".to_string(), *span))
}
