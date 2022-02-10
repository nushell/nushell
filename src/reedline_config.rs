use crate::is_perf_true;
use crossterm::event::{KeyCode, KeyModifiers};
use log::info;
use nu_color_config::lookup_ansi_color_style;
use nu_protocol::{extract_value, Config, ParsedKeybinding, ShellError, Span, Type, Value};
use reedline::{
    default_emacs_keybindings, default_vi_insert_keybindings, default_vi_normal_keybindings,
    CompletionMenu, EditCommand, HistoryMenu, Keybindings, Reedline, ReedlineEvent,
};

// Creates an input object for the completion menu based on the dictionary
// stored in the config variable
pub(crate) fn add_completion_menu(line_editor: Reedline, config: &Config) -> Reedline {
    let mut completion_menu = CompletionMenu::default();

    completion_menu = match config
        .menu_config
        .get("columns")
        .and_then(|value| value.as_integer().ok())
    {
        Some(value) => completion_menu.with_columns(value as u16),
        None => completion_menu,
    };

    completion_menu = completion_menu.with_column_width(
        config
            .menu_config
            .get("col-width")
            .and_then(|value| value.as_integer().ok())
            .map(|value| value as usize),
    );

    completion_menu = match config
        .menu_config
        .get("col-padding")
        .and_then(|value| value.as_integer().ok())
    {
        Some(value) => completion_menu.with_column_padding(value as usize),
        None => completion_menu,
    };

    completion_menu = match config
        .menu_config
        .get("text-style")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => completion_menu.with_text_style(lookup_ansi_color_style(&value)),
        None => completion_menu,
    };

    completion_menu = match config
        .menu_config
        .get("selected-text-style")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => completion_menu.with_selected_text_style(lookup_ansi_color_style(&value)),
        None => completion_menu,
    };

    completion_menu = match config
        .menu_config
        .get("marker")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => completion_menu.with_marker(value),
        None => completion_menu,
    };

    let ret_val = line_editor.with_menu(Box::new(completion_menu));
    if is_perf_true() {
        info!("add-completion-menu {}:{}:{}", file!(), line!(), column!());
    }

    ret_val
}

// Creates an input object for the history menu based on the dictionary
// stored in the config variable
pub(crate) fn add_history_menu(line_editor: Reedline, config: &Config) -> Reedline {
    let mut history_menu = HistoryMenu::default();

    history_menu = match config
        .history_config
        .get("page-size")
        .and_then(|value| value.as_integer().ok())
    {
        Some(value) => history_menu.with_page_size(value as usize),
        None => history_menu,
    };

    history_menu = match config
        .history_config
        .get("selector")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => {
            let char = value.chars().next().unwrap_or(':');
            history_menu.with_row_char(char)
        }
        None => history_menu,
    };

    history_menu = match config
        .history_config
        .get("text-style")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => history_menu.with_text_style(lookup_ansi_color_style(&value)),
        None => history_menu,
    };

    history_menu = match config
        .history_config
        .get("selected-text-style")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => history_menu.with_selected_text_style(lookup_ansi_color_style(&value)),
        None => history_menu,
    };

    history_menu = match config
        .history_config
        .get("marker")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => history_menu.with_marker(value),
        None => history_menu,
    };

    let ret_val = line_editor.with_menu(Box::new(history_menu));
    if is_perf_true() {
        info!("add_history_menu {}:{}:{}", file!(), line!(), column!());
    }

    ret_val
}

fn add_menu_keybindings(keybindings: &mut Keybindings) {
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('x'),
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("history-menu".to_string()),
            ReedlineEvent::MenuPageNext,
        ]),
    );

    keybindings.add_binding(
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        KeyCode::Char('x'),
        ReedlineEvent::MenuPagePrevious,
    );

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion-menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::MenuPrevious,
    );

    if is_perf_true() {
        info!("add-menu-keybindings {}:{}:{}", file!(), line!(), column!());
    }
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
            add_menu_keybindings(&mut keybindings);

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

            if is_perf_true() {
                info!(
                    "create_keybindings (emacs) {}:{}:{}",
                    file!(),
                    line!(),
                    column!()
                );
            }

            Ok(KeybindingsMode::Emacs(keybindings))
        }
        _ => {
            let mut insert_keybindings = default_vi_insert_keybindings();
            let mut normal_keybindings = default_vi_normal_keybindings();

            add_menu_keybindings(&mut insert_keybindings);
            add_menu_keybindings(&mut normal_keybindings);

            for parsed_keybinding in parsed_keybindings {
                if parsed_keybinding.mode.into_string("", config).as_str() == "vi-insert" {
                    add_keybinding(&mut insert_keybindings, parsed_keybinding, config)?
                } else if parsed_keybinding.mode.into_string("", config).as_str() == "vi-normal" {
                    add_keybinding(&mut normal_keybindings, parsed_keybinding, config)?
                }
            }

            if is_perf_true() {
                info!(
                    "create_keybindings (vi) {}:{}:{}",
                    file!(),
                    line!(),
                    column!()
                );
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
                "CONTROL, SHIFT, ALT or NONE".to_string(),
                keybinding.modifier.into_abbreviated_string(config),
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
            let mut char_iter = c.chars().skip(5);
            let pos1 = char_iter.next();
            let pos2 = char_iter.next();

            let char = match (pos1, pos2) {
                (Some(char), None) => Ok(char),
                _ => Err(ShellError::UnsupportedConfigValue(
                    "char_<CHAR: unicode codepoint>".to_string(),
                    c.to_string(),
                    keybinding.keycode.span()?,
                )),
            }?;

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
        c if c.starts_with('f') => {
            let fn_num: u8 = c[1..]
                .parse()
                .ok()
                .filter(|num| matches!(num, 1..=12))
                .ok_or(ShellError::UnsupportedConfigValue(
                    "(f1|f2|...|f12)".to_string(),
                    format!("unknown function key: {}", c),
                    keybinding.keycode.span()?,
                ))?;
            KeyCode::F(fn_num)
        }
        "null" => KeyCode::Null,
        "esc" | "escape" => KeyCode::Esc,
        _ => {
            return Err(ShellError::UnsupportedConfigValue(
                "crossterm KeyCode".to_string(),
                keybinding.keycode.into_abbreviated_string(config),
                keybinding.keycode.span()?,
            ))
        }
    };

    let event = parse_event(keybinding.event.clone(), config)?;

    keybindings.add_binding(modifier, keycode, event);

    if is_perf_true() {
        info!("add_keybinding {}:{}:{}", file!(), line!(), column!());
    }

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
                    "historyhintcomplete" => ReedlineEvent::HistoryHintComplete,
                    "historyhintwordcomplete" => ReedlineEvent::HistoryHintWordComplete,
                    "ctrld" => ReedlineEvent::CtrlD,
                    "ctrlc" => ReedlineEvent::CtrlC,
                    "enter" => ReedlineEvent::Enter,
                    "esc" | "escape" => ReedlineEvent::Esc,
                    "up" => ReedlineEvent::Up,
                    "down" => ReedlineEvent::Down,
                    "right" => ReedlineEvent::Right,
                    "left" => ReedlineEvent::Left,
                    "searchhistory" => ReedlineEvent::SearchHistory,
                    "nexthistory" => ReedlineEvent::NextHistory,
                    "previoushistory" => ReedlineEvent::PreviousHistory,
                    "repaint" => ReedlineEvent::Repaint,
                    "menudown" => ReedlineEvent::MenuDown,
                    "menuup" => ReedlineEvent::MenuUp,
                    "menuleft" => ReedlineEvent::MenuLeft,
                    "menuright" => ReedlineEvent::MenuRight,
                    "menunext" => ReedlineEvent::MenuNext,
                    "menuprevious" => ReedlineEvent::MenuPrevious,
                    "menupagenext" => ReedlineEvent::MenuPageNext,
                    "menupageprevious" => ReedlineEvent::MenuPagePrevious,
                    "menu" => {
                        let menu = extract_value("name", &cols, &vals, &span)?;
                        ReedlineEvent::Menu(menu.into_string("", config))
                    }
                    "edit" => {
                        let edit = extract_value("edit", &cols, &vals, &span)?;
                        let edit = parse_edit(edit, config)?;

                        ReedlineEvent::Edit(vec![edit])
                    }
                    v => {
                        return Err(ShellError::UnsupportedConfigValue(
                            "Reedline event".to_string(),
                            v.to_string(),
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

            if is_perf_true() {
                info!("parse_event (record) {}:{}:{}", file!(), line!(), column!());
            }

            Ok(event)
        }
        Value::List { vals, .. } => {
            // If all the elements in the list are lists, then they represent an UntilFound event.
            // This means that only one of the parsed events from the list will be executed.
            // Otherwise, the expect shape should be lists of records which indicates a sequence
            // of events that will happen one after the other
            let until_found = vals.iter().all(|v| matches!(v.get_type(), Type::List(..)));
            let events = vals
                .into_iter()
                .map(|value| parse_event(value, config))
                .collect::<Result<Vec<ReedlineEvent>, ShellError>>()?;

            if is_perf_true() {
                info!("parse_event (list) {}:{}:{}", file!(), line!(), column!());
            }

            if until_found {
                Ok(ReedlineEvent::UntilFound(events))
            } else {
                Ok(ReedlineEvent::Multiple(events))
            }
        }
        v => Err(ShellError::UnsupportedConfigValue(
            "record or list of records".to_string(),
            v.into_abbreviated_string(config),
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
                        "reedline EditCommand".to_string(),
                        e.to_string(),
                        edit.span()?,
                    ))
                }
            }
        }
        e => {
            return Err(ShellError::UnsupportedConfigValue(
                "record with EditCommand".to_string(),
                e.into_abbreviated_string(config),
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
