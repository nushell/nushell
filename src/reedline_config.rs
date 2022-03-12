use crossterm::event::{KeyCode, KeyModifiers};
use nu_color_config::lookup_ansi_color_style;
use nu_protocol::{extract_value, Config, ParsedKeybinding, ShellError, Span, Value};
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
            .get("col_width")
            .and_then(|value| value.as_integer().ok())
            .map(|value| value as usize),
    );

    completion_menu = match config
        .menu_config
        .get("col_padding")
        .and_then(|value| value.as_integer().ok())
    {
        Some(value) => completion_menu.with_column_padding(value as usize),
        None => completion_menu,
    };

    completion_menu = match config
        .menu_config
        .get("text_style")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => completion_menu.with_text_style(lookup_ansi_color_style(&value)),
        None => completion_menu,
    };

    completion_menu = match config
        .menu_config
        .get("selected_text_style")
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

    line_editor.with_menu(Box::new(completion_menu))
}

// Creates an input object for the history menu based on the dictionary
// stored in the config variable
pub(crate) fn add_history_menu(line_editor: Reedline, config: &Config) -> Reedline {
    let mut history_menu = HistoryMenu::default();

    history_menu = match config
        .history_config
        .get("page_size")
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
            let char = value.chars().next().unwrap_or('!');
            history_menu.with_selection_char(char)
        }
        None => history_menu,
    };

    history_menu = match config
        .history_config
        .get("text_style")
        .and_then(|value| value.as_string().ok())
    {
        Some(value) => history_menu.with_text_style(lookup_ansi_color_style(&value)),
        None => history_menu,
    };

    history_menu = match config
        .history_config
        .get("selected_text_style")
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

    line_editor.with_menu(Box::new(history_menu))
}

fn add_menu_keybindings(keybindings: &mut Keybindings) {
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('x'),
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("history_menu".to_string()),
            ReedlineEvent::MenuPageNext,
        ]),
    );

    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('z'),
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::MenuPagePrevious,
            ReedlineEvent::Edit(vec![EditCommand::Undo]),
        ]),
    );

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::MenuPrevious,
    );
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

    let mut emacs_keybindings = default_emacs_keybindings();
    let mut insert_keybindings = default_vi_insert_keybindings();
    let mut normal_keybindings = default_vi_normal_keybindings();

    for keybinding in parsed_keybindings {
        add_keybinding(
            &keybinding.mode,
            keybinding,
            config,
            &mut emacs_keybindings,
            &mut insert_keybindings,
            &mut normal_keybindings,
        )?
    }

    match config.edit_mode.as_str() {
        "emacs" => {
            add_menu_keybindings(&mut emacs_keybindings);

            Ok(KeybindingsMode::Emacs(emacs_keybindings))
        }
        _ => {
            add_menu_keybindings(&mut insert_keybindings);
            add_menu_keybindings(&mut normal_keybindings);

            Ok(KeybindingsMode::Vi {
                insert_keybindings,
                normal_keybindings,
            })
        }
    }
}

fn add_keybinding(
    mode: &Value,
    keybinding: &ParsedKeybinding,
    config: &Config,
    emacs_keybindings: &mut Keybindings,
    insert_keybindings: &mut Keybindings,
    normal_keybindings: &mut Keybindings,
) -> Result<(), ShellError> {
    match &mode {
        Value::String { val, span } => match val.as_str() {
            "emacs" => add_parsed_keybinding(emacs_keybindings, keybinding, config),
            "vi_insert" => add_parsed_keybinding(insert_keybindings, keybinding, config),
            "vi_normal" => add_parsed_keybinding(normal_keybindings, keybinding, config),
            m => Err(ShellError::UnsupportedConfigValue(
                "emacs, vi_insert or vi_normal".to_string(),
                m.to_string(),
                *span,
            )),
        },
        Value::List { vals, .. } => {
            for inner_mode in vals {
                add_keybinding(
                    inner_mode,
                    keybinding,
                    config,
                    emacs_keybindings,
                    insert_keybindings,
                    normal_keybindings,
                )?
            }

            Ok(())
        }
        v => Err(ShellError::UnsupportedConfigValue(
            "string or list of strings".to_string(),
            v.into_abbreviated_string(config),
            v.span()?,
        )),
    }
}

fn add_parsed_keybinding(
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

    let event = parse_event(&keybinding.event, config)?;

    keybindings.add_binding(modifier, keycode, event);

    Ok(())
}

enum EventType<'config> {
    Send(&'config Value),
    Edit(&'config Value),
    Until(&'config Value),
}

impl<'config> EventType<'config> {
    fn try_from_columns(
        cols: &'config [String],
        vals: &'config [Value],
        span: &'config Span,
    ) -> Result<Self, ShellError> {
        extract_value("send", cols, vals, span)
            .map(Self::Send)
            .or_else(|_| extract_value("edit", cols, vals, span).map(Self::Edit))
            .or_else(|_| extract_value("until", cols, vals, span).map(Self::Until))
            .map_err(|_| ShellError::MissingConfigValue("send, edit or until".to_string(), *span))
    }
}

fn parse_event(value: &Value, config: &Config) -> Result<ReedlineEvent, ShellError> {
    match value {
        Value::Record { cols, vals, span } => {
            match EventType::try_from_columns(cols, vals, span)? {
                EventType::Send(value) => event_from_record(
                    value.into_string("", config).to_lowercase().as_str(),
                    cols,
                    vals,
                    config,
                    span,
                ),
                EventType::Edit(value) => {
                    let edit = parse_edit(value, config)?;
                    Ok(ReedlineEvent::Edit(vec![edit]))
                }
                EventType::Until(value) => match value {
                    Value::List { vals, .. } => {
                        let events = vals
                            .iter()
                            .map(|value| parse_event(value, config))
                            .collect::<Result<Vec<ReedlineEvent>, ShellError>>()?;

                        Ok(ReedlineEvent::UntilFound(events))
                    }
                    v => Err(ShellError::UnsupportedConfigValue(
                        "list of events".to_string(),
                        v.into_abbreviated_string(config),
                        v.span()?,
                    )),
                },
            }
        }
        Value::List { vals, .. } => {
            let events = vals
                .iter()
                .map(|value| parse_event(value, config))
                .collect::<Result<Vec<ReedlineEvent>, ShellError>>()?;

            Ok(ReedlineEvent::Multiple(events))
        }
        v => Err(ShellError::UnsupportedConfigValue(
            "record or list of records".to_string(),
            v.into_abbreviated_string(config),
            v.span()?,
        )),
    }
}

fn event_from_record(
    name: &str,
    cols: &[String],
    vals: &[Value],
    config: &Config,
    span: &Span,
) -> Result<ReedlineEvent, ShellError> {
    match name {
        "none" => Ok(ReedlineEvent::None),
        "actionhandler" => Ok(ReedlineEvent::ActionHandler),
        "clearscreen" => Ok(ReedlineEvent::ClearScreen),
        "historyhintcomplete" => Ok(ReedlineEvent::HistoryHintComplete),
        "historyhintwordcomplete" => Ok(ReedlineEvent::HistoryHintWordComplete),
        "ctrld" => Ok(ReedlineEvent::CtrlD),
        "ctrlc" => Ok(ReedlineEvent::CtrlC),
        "enter" => Ok(ReedlineEvent::Enter),
        "esc" | "escape" => Ok(ReedlineEvent::Esc),
        "up" => Ok(ReedlineEvent::Up),
        "down" => Ok(ReedlineEvent::Down),
        "right" => Ok(ReedlineEvent::Right),
        "left" => Ok(ReedlineEvent::Left),
        "searchhistory" => Ok(ReedlineEvent::SearchHistory),
        "nexthistory" => Ok(ReedlineEvent::NextHistory),
        "previoushistory" => Ok(ReedlineEvent::PreviousHistory),
        "repaint" => Ok(ReedlineEvent::Repaint),
        "menudown" => Ok(ReedlineEvent::MenuDown),
        "menuup" => Ok(ReedlineEvent::MenuUp),
        "menuleft" => Ok(ReedlineEvent::MenuLeft),
        "menuright" => Ok(ReedlineEvent::MenuRight),
        "menunext" => Ok(ReedlineEvent::MenuNext),
        "menuprevious" => Ok(ReedlineEvent::MenuPrevious),
        "menupagenext" => Ok(ReedlineEvent::MenuPageNext),
        "menupageprevious" => Ok(ReedlineEvent::MenuPagePrevious),
        "menu" => {
            let menu = extract_value("name", cols, vals, span)?;
            Ok(ReedlineEvent::Menu(menu.into_string("", config)))
        }
        "executehostcommand" => {
            let cmd = extract_value("cmd", cols, vals, span)?;
            Ok(ReedlineEvent::ExecuteHostCommand(
                cmd.into_string("", config),
            ))
        }
        v => Err(ShellError::UnsupportedConfigValue(
            "Reedline event".to_string(),
            v.to_string(),
            *span,
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_send_event() {
        let cols = vec!["send".to_string()];
        let vals = vec![Value::String {
            val: "Enter".to_string(),
            span: Span::test_data(),
        }];

        let span = Span::test_data();
        let b = EventType::try_from_columns(&cols, &vals, &span).unwrap();
        assert!(matches!(b, EventType::Send(_)));

        let event = Value::Record {
            vals,
            cols,
            span: Span::test_data(),
        };
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(parsed_event, ReedlineEvent::Enter);
    }

    #[test]
    fn test_edit_event() {
        let cols = vec!["edit".to_string()];
        let vals = vec![Value::Record {
            cols: vec!["cmd".to_string()],
            vals: vec![Value::String {
                val: "Clear".to_string(),
                span: Span::test_data(),
            }],
            span: Span::test_data(),
        }];

        let span = Span::test_data();
        let b = EventType::try_from_columns(&cols, &vals, &span).unwrap();
        assert!(matches!(b, EventType::Edit(_)));

        let event = Value::Record {
            vals,
            cols,
            span: Span::test_data(),
        };
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(parsed_event, ReedlineEvent::Edit(vec![EditCommand::Clear]));
    }

    #[test]
    fn test_send_menu() {
        let cols = vec!["send".to_string(), "name".to_string()];
        let vals = vec![
            Value::String {
                val: "Menu".to_string(),
                span: Span::test_data(),
            },
            Value::String {
                val: "history_menu".to_string(),
                span: Span::test_data(),
            },
        ];

        let span = Span::test_data();
        let b = EventType::try_from_columns(&cols, &vals, &span).unwrap();
        assert!(matches!(b, EventType::Send(_)));

        let event = Value::Record {
            vals,
            cols,
            span: Span::test_data(),
        };
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            ReedlineEvent::Menu("history_menu".to_string())
        );
    }

    #[test]
    fn test_until_event() {
        // Menu event
        let cols = vec!["send".to_string(), "name".to_string()];
        let vals = vec![
            Value::String {
                val: "Menu".to_string(),
                span: Span::test_data(),
            },
            Value::String {
                val: "history_menu".to_string(),
                span: Span::test_data(),
            },
        ];

        let menu_event = Value::Record {
            cols,
            vals,
            span: Span::test_data(),
        };

        // Enter event
        let cols = vec!["send".to_string()];
        let vals = vec![Value::String {
            val: "Enter".to_string(),
            span: Span::test_data(),
        }];

        let enter_event = Value::Record {
            cols,
            vals,
            span: Span::test_data(),
        };

        // Until event
        let cols = vec!["until".to_string()];
        let vals = vec![Value::List {
            vals: vec![menu_event, enter_event],
            span: Span::test_data(),
        }];

        let span = Span::test_data();
        let b = EventType::try_from_columns(&cols, &vals, &span).unwrap();
        assert!(matches!(b, EventType::Until(_)));

        let event = Value::Record {
            cols,
            vals,
            span: Span::test_data(),
        };
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("history_menu".to_string()),
                ReedlineEvent::Enter,
            ])
        );
    }

    #[test]
    fn test_multiple_event() {
        // Menu event
        let cols = vec!["send".to_string(), "name".to_string()];
        let vals = vec![
            Value::String {
                val: "Menu".to_string(),
                span: Span::test_data(),
            },
            Value::String {
                val: "history_menu".to_string(),
                span: Span::test_data(),
            },
        ];

        let menu_event = Value::Record {
            cols,
            vals,
            span: Span::test_data(),
        };

        // Enter event
        let cols = vec!["send".to_string()];
        let vals = vec![Value::String {
            val: "Enter".to_string(),
            span: Span::test_data(),
        }];

        let enter_event = Value::Record {
            cols,
            vals,
            span: Span::test_data(),
        };

        // Multiple event
        let event = Value::List {
            vals: vec![menu_event, enter_event],
            span: Span::test_data(),
        };

        let config = Config::default();
        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            ReedlineEvent::Multiple(vec![
                ReedlineEvent::Menu("history_menu".to_string()),
                ReedlineEvent::Enter,
            ])
        );
    }

    #[test]
    fn test_error() {
        let cols = vec!["not_exist".to_string()];
        let vals = vec![Value::String {
            val: "Enter".to_string(),
            span: Span::test_data(),
        }];

        let span = Span::test_data();
        let b = EventType::try_from_columns(&cols, &vals, &span);
        assert!(matches!(b, Err(ShellError::MissingConfigValue(_, _))));
    }
}
