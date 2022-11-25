use super::DescriptionMenu;
use crate::{menus::NuMenuCompleter, NuHelpCompleter};
use crossterm::event::{KeyCode, KeyModifiers};
use nu_color_config::lookup_ansi_color_style;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    color_value_string, create_menus,
    engine::{EngineState, Stack, StateWorkingSet},
    extract_value, Config, IntoPipelineData, ParsedKeybinding, ParsedMenu, PipelineData,
    ShellError, Span, Value,
};
use reedline::{
    default_emacs_keybindings, default_vi_insert_keybindings, default_vi_normal_keybindings,
    ColumnarMenu, EditCommand, Keybindings, ListMenu, Reedline, ReedlineEvent, ReedlineMenu,
};
use std::sync::Arc;

const DEFAULT_COMPLETION_MENU: &str = r#"
{
  name: completion_menu
  only_buffer_difference: false
  marker: "| "
  type: {
      layout: columnar
      columns: 4
      col_width: 20
      col_padding: 2
  }
  style: {
      text: green,
      selected_text: green_reverse
      description_text: yellow
  }
}"#;

const DEFAULT_HISTORY_MENU: &str = r#"
{
  name: history_menu
  only_buffer_difference: true
  marker: "? "
  type: {
      layout: list
      page_size: 10
  }
  style: {
      text: green,
      selected_text: green_reverse
      description_text: yellow
  }
}"#;

const DEFAULT_HELP_MENU: &str = r#"
{
  name: help_menu
  only_buffer_difference: true
  marker: "? "
  type: {
      layout: description
      columns: 4
      col_width: 20
      col_padding: 2
      selection_rows: 4
      description_rows: 10
  }
  style: {
      text: green,
      selected_text: green_reverse
      description_text: yellow
  }
}"#;

// Adds all menus to line editor
pub(crate) fn add_menus(
    mut line_editor: Reedline,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: &Config,
) -> Result<Reedline, ShellError> {
    line_editor = line_editor.clear_menus();

    for menu in &config.menus {
        line_editor = add_menu(line_editor, menu, engine_state.clone(), stack, config)?
    }

    // Checking if the default menus have been added from the config file
    let default_menus = vec![
        ("completion_menu", DEFAULT_COMPLETION_MENU),
        ("history_menu", DEFAULT_HISTORY_MENU),
        ("help_menu", DEFAULT_HELP_MENU),
    ];

    for (name, definition) in default_menus {
        if !config
            .menus
            .iter()
            .any(|menu| menu.name.into_string("", config) == name)
        {
            let (block, _) = {
                let mut working_set = StateWorkingSet::new(&engine_state);
                let (output, _) = parse(
                    &mut working_set,
                    Some(name), // format!("entry #{}", entry_num)
                    definition.as_bytes(),
                    true,
                    &[],
                );

                (output, working_set.render())
            };

            let mut temp_stack = Stack::new();
            let input = Value::nothing(Span::test_data()).into_pipeline_data();
            let res = eval_block(&engine_state, &mut temp_stack, &block, input, false, false)?;

            if let PipelineData::Value(value, None) = res {
                for menu in create_menus(&value)? {
                    line_editor =
                        add_menu(line_editor, &menu, engine_state.clone(), stack, config)?;
                }
            }
        }
    }

    Ok(line_editor)
}

fn add_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: &Config,
) -> Result<Reedline, ShellError> {
    if let Value::Record { cols, vals, span } = &menu.menu_type {
        let layout = extract_value("layout", cols, vals, span)?.into_string("", config);

        match layout.as_str() {
            "columnar" => add_columnar_menu(line_editor, menu, engine_state, stack, config),
            "list" => add_list_menu(line_editor, menu, engine_state, stack, config),
            "description" => add_description_menu(line_editor, menu, engine_state, stack, config),
            _ => Err(ShellError::UnsupportedConfigValue(
                "columnar, list or description".to_string(),
                menu.menu_type.into_abbreviated_string(config),
                menu.menu_type.span()?,
            )),
        }
    } else {
        Err(ShellError::UnsupportedConfigValue(
            "only record type".to_string(),
            menu.menu_type.into_abbreviated_string(config),
            menu.menu_type.span()?,
        ))
    }
}

macro_rules! add_style {
    // first arm match add!(1,2), add!(2,3) etc
    ($name:expr, $cols: expr, $vals:expr, $span:expr, $config: expr, $menu:expr, $f:expr) => {
        $menu = match extract_value($name, $cols, $vals, $span) {
            Ok(text) => {
                let text = match text {
                    Value::String { val, .. } => val.clone(),
                    Value::Record { cols, vals, span } => {
                        color_value_string(span, cols, vals, $config).into_string("", $config)
                    }
                    _ => "green".to_string(),
                };
                let style = lookup_ansi_color_style(&text);
                $f($menu, style)
            }
            Err(_) => $menu,
        };
    };
}

// Adds a columnar menu to the editor engine
pub(crate) fn add_columnar_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: &Config,
) -> Result<Reedline, ShellError> {
    let name = menu.name.into_string("", config);
    let mut columnar_menu = ColumnarMenu::default().with_name(&name);

    if let Value::Record { cols, vals, span } = &menu.menu_type {
        columnar_menu = match extract_value("columns", cols, vals, span) {
            Ok(columns) => {
                let columns = columns.as_integer()?;
                columnar_menu.with_columns(columns as u16)
            }
            Err(_) => columnar_menu,
        };

        columnar_menu = match extract_value("col_width", cols, vals, span) {
            Ok(col_width) => {
                let col_width = col_width.as_integer()?;
                columnar_menu.with_column_width(Some(col_width as usize))
            }
            Err(_) => columnar_menu.with_column_width(None),
        };

        columnar_menu = match extract_value("col_padding", cols, vals, span) {
            Ok(col_padding) => {
                let col_padding = col_padding.as_integer()?;
                columnar_menu.with_column_padding(col_padding as usize)
            }
            Err(_) => columnar_menu,
        };
    }

    if let Value::Record { cols, vals, span } = &menu.style {
        add_style!(
            "text",
            cols,
            vals,
            span,
            config,
            columnar_menu,
            ColumnarMenu::with_text_style
        );
        add_style!(
            "selected_text",
            cols,
            vals,
            span,
            config,
            columnar_menu,
            ColumnarMenu::with_selected_text_style
        );
        add_style!(
            "description_text",
            cols,
            vals,
            span,
            config,
            columnar_menu,
            ColumnarMenu::with_description_text_style
        );
    }

    let marker = menu.marker.into_string("", config);
    columnar_menu = columnar_menu.with_marker(marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    columnar_menu = columnar_menu.with_only_buffer_difference(only_buffer_difference);

    match &menu.source {
        Value::Nothing { .. } => {
            Ok(line_editor.with_menu(ReedlineMenu::EngineCompleter(Box::new(columnar_menu))))
        }
        Value::Closure {
            val,
            captures,
            span,
        } => {
            let menu_completer = NuMenuCompleter::new(
                *val,
                *span,
                stack.captures_to_stack(captures),
                engine_state,
                only_buffer_difference,
            );
            Ok(line_editor.with_menu(ReedlineMenu::WithCompleter {
                menu: Box::new(columnar_menu),
                completer: Box::new(menu_completer),
            }))
        }
        _ => Err(ShellError::UnsupportedConfigValue(
            "block or omitted value".to_string(),
            menu.source.into_abbreviated_string(config),
            menu.source.span()?,
        )),
    }
}

// Adds a search menu to the line editor
pub(crate) fn add_list_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: &Config,
) -> Result<Reedline, ShellError> {
    let name = menu.name.into_string("", config);
    let mut list_menu = ListMenu::default().with_name(&name);

    if let Value::Record { cols, vals, span } = &menu.menu_type {
        list_menu = match extract_value("page_size", cols, vals, span) {
            Ok(page_size) => {
                let page_size = page_size.as_integer()?;
                list_menu.with_page_size(page_size as usize)
            }
            Err(_) => list_menu,
        };
    }

    if let Value::Record { cols, vals, span } = &menu.style {
        add_style!(
            "text",
            cols,
            vals,
            span,
            config,
            list_menu,
            ListMenu::with_text_style
        );
        add_style!(
            "selected_text",
            cols,
            vals,
            span,
            config,
            list_menu,
            ListMenu::with_selected_text_style
        );
        add_style!(
            "description_text",
            cols,
            vals,
            span,
            config,
            list_menu,
            ListMenu::with_description_text_style
        );
    }

    let marker = menu.marker.into_string("", config);
    list_menu = list_menu.with_marker(marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    list_menu = list_menu.with_only_buffer_difference(only_buffer_difference);

    match &menu.source {
        Value::Nothing { .. } => {
            Ok(line_editor.with_menu(ReedlineMenu::HistoryMenu(Box::new(list_menu))))
        }
        Value::Closure {
            val,
            captures,
            span,
        } => {
            let menu_completer = NuMenuCompleter::new(
                *val,
                *span,
                stack.captures_to_stack(captures),
                engine_state,
                only_buffer_difference,
            );
            Ok(line_editor.with_menu(ReedlineMenu::WithCompleter {
                menu: Box::new(list_menu),
                completer: Box::new(menu_completer),
            }))
        }
        _ => Err(ShellError::UnsupportedConfigValue(
            "block or omitted value".to_string(),
            menu.source.into_abbreviated_string(config),
            menu.source.span()?,
        )),
    }
}

// Adds a description menu to the line editor
pub(crate) fn add_description_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: &Config,
) -> Result<Reedline, ShellError> {
    let name = menu.name.into_string("", config);
    let mut description_menu = DescriptionMenu::default().with_name(&name);

    if let Value::Record { cols, vals, span } = &menu.menu_type {
        description_menu = match extract_value("columns", cols, vals, span) {
            Ok(columns) => {
                let columns = columns.as_integer()?;
                description_menu.with_columns(columns as u16)
            }
            Err(_) => description_menu,
        };

        description_menu = match extract_value("col_width", cols, vals, span) {
            Ok(col_width) => {
                let col_width = col_width.as_integer()?;
                description_menu.with_column_width(Some(col_width as usize))
            }
            Err(_) => description_menu.with_column_width(None),
        };

        description_menu = match extract_value("col_padding", cols, vals, span) {
            Ok(col_padding) => {
                let col_padding = col_padding.as_integer()?;
                description_menu.with_column_padding(col_padding as usize)
            }
            Err(_) => description_menu,
        };

        description_menu = match extract_value("selection_rows", cols, vals, span) {
            Ok(selection_rows) => {
                let selection_rows = selection_rows.as_integer()?;
                description_menu.with_selection_rows(selection_rows as u16)
            }
            Err(_) => description_menu,
        };

        description_menu = match extract_value("description_rows", cols, vals, span) {
            Ok(description_rows) => {
                let description_rows = description_rows.as_integer()?;
                description_menu.with_description_rows(description_rows as usize)
            }
            Err(_) => description_menu,
        };
    }

    if let Value::Record { cols, vals, span } = &menu.style {
        add_style!(
            "text",
            cols,
            vals,
            span,
            config,
            description_menu,
            DescriptionMenu::with_text_style
        );
        add_style!(
            "selected_text",
            cols,
            vals,
            span,
            config,
            description_menu,
            DescriptionMenu::with_selected_text_style
        );
        add_style!(
            "description_text",
            cols,
            vals,
            span,
            config,
            description_menu,
            DescriptionMenu::with_description_text_style
        );
    }

    let marker = menu.marker.into_string("", config);
    description_menu = description_menu.with_marker(marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    description_menu = description_menu.with_only_buffer_difference(only_buffer_difference);

    match &menu.source {
        Value::Nothing { .. } => {
            let completer = Box::new(NuHelpCompleter::new(engine_state));
            Ok(line_editor.with_menu(ReedlineMenu::WithCompleter {
                menu: Box::new(description_menu),
                completer,
            }))
        }
        Value::Closure {
            val,
            captures,
            span,
        } => {
            let menu_completer = NuMenuCompleter::new(
                *val,
                *span,
                stack.captures_to_stack(captures),
                engine_state,
                only_buffer_difference,
            );
            Ok(line_editor.with_menu(ReedlineMenu::WithCompleter {
                menu: Box::new(description_menu),
                completer: Box::new(menu_completer),
            }))
        }
        _ => Err(ShellError::UnsupportedConfigValue(
            "closure or omitted value".to_string(),
            menu.source.into_abbreviated_string(config),
            menu.source.span()?,
        )),
    }
}

fn add_menu_keybindings(keybindings: &mut Keybindings) {
    // Completer menu keybindings
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::Edit(vec![EditCommand::Complete]),
        ]),
    );

    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::MenuPrevious,
    );

    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('r'),
        ReedlineEvent::Menu("history_menu".to_string()),
    );

    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('x'),
        ReedlineEvent::MenuPageNext,
    );

    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('z'),
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::MenuPagePrevious,
            ReedlineEvent::Edit(vec![EditCommand::Undo]),
        ]),
    );

    // Help menu keybinding
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::F(1),
        ReedlineEvent::Menu("help_menu".to_string()),
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

    match config.edit_mode.as_str() {
        "emacs" => {
            add_menu_keybindings(&mut emacs_keybindings);
        }
        _ => {
            add_menu_keybindings(&mut insert_keybindings);
            add_menu_keybindings(&mut normal_keybindings);
        }
    }
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
        "emacs" => Ok(KeybindingsMode::Emacs(emacs_keybindings)),
        _ => Ok(KeybindingsMode::Vi {
            insert_keybindings,
            normal_keybindings,
        }),
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
        "space" => KeyCode::Char(' '),
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
    if let Some(event) = parse_event(&keybinding.event, config)? {
        keybindings.add_binding(modifier, keycode, event);
    } else {
        keybindings.remove_binding(modifier, keycode);
    }

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

fn parse_event(value: &Value, config: &Config) -> Result<Option<ReedlineEvent>, ShellError> {
    match value {
        Value::Record { cols, vals, span } => {
            match EventType::try_from_columns(cols, vals, span)? {
                EventType::Send(value) => event_from_record(
                    value.into_string("", config).to_lowercase().as_str(),
                    cols,
                    vals,
                    config,
                    span,
                )
                .map(Some),
                EventType::Edit(value) => {
                    let edit = edit_from_record(
                        value.into_string("", config).to_lowercase().as_str(),
                        cols,
                        vals,
                        config,
                        span,
                    )?;
                    Ok(Some(ReedlineEvent::Edit(vec![edit])))
                }
                EventType::Until(value) => match value {
                    Value::List { vals, .. } => {
                        let events = vals
                            .iter()
                            .map(|value| match parse_event(value, config) {
                                Ok(inner) => match inner {
                                    None => Err(ShellError::UnsupportedConfigValue(
                                        "List containing valid events".to_string(),
                                        "Nothing value (null)".to_string(),
                                        value.span()?,
                                    )),
                                    Some(event) => Ok(event),
                                },
                                Err(e) => Err(e),
                            })
                            .collect::<Result<Vec<ReedlineEvent>, ShellError>>()?;

                        Ok(Some(ReedlineEvent::UntilFound(events)))
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
                .map(|value| match parse_event(value, config) {
                    Ok(inner) => match inner {
                        None => Err(ShellError::UnsupportedConfigValue(
                            "List containing valid events".to_string(),
                            "Nothing value (null)".to_string(),
                            value.span()?,
                        )),
                        Some(event) => Ok(event),
                    },
                    Err(e) => Err(e),
                })
                .collect::<Result<Vec<ReedlineEvent>, ShellError>>()?;

            Ok(Some(ReedlineEvent::Multiple(events)))
        }
        Value::Nothing { .. } => Ok(None),
        v => Err(ShellError::UnsupportedConfigValue(
            "record or list of records, null to unbind key".to_string(),
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
    let event = match name {
        "none" => ReedlineEvent::None,
        "clearscreen" => ReedlineEvent::ClearScreen,
        "clearscrollback" => ReedlineEvent::ClearScrollback,
        "historyhintcomplete" => ReedlineEvent::HistoryHintComplete,
        "historyhintwordcomplete" => ReedlineEvent::HistoryHintWordComplete,
        "ctrld" => ReedlineEvent::CtrlD,
        "ctrlc" => ReedlineEvent::CtrlC,
        "enter" => ReedlineEvent::Enter,
        "submit" => ReedlineEvent::Submit,
        "submitornewline" => ReedlineEvent::SubmitOrNewline,
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
        "openeditor" => ReedlineEvent::OpenEditor,
        "menu" => {
            let menu = extract_value("name", cols, vals, span)?;
            ReedlineEvent::Menu(menu.into_string("", config))
        }
        "executehostcommand" => {
            let cmd = extract_value("cmd", cols, vals, span)?;
            ReedlineEvent::ExecuteHostCommand(cmd.into_string("", config))
        }
        v => {
            return Err(ShellError::UnsupportedConfigValue(
                "Reedline event".to_string(),
                v.to_string(),
                *span,
            ))
        }
    };

    Ok(event)
}

fn edit_from_record(
    name: &str,
    cols: &[String],
    vals: &[Value],
    config: &Config,
    span: &Span,
) -> Result<EditCommand, ShellError> {
    let edit = match name {
        "movetostart" => EditCommand::MoveToStart,
        "movetolinestart" => EditCommand::MoveToLineStart,
        "movetoend" => EditCommand::MoveToEnd,
        "movetolineend" => EditCommand::MoveToLineEnd,
        "moveleft" => EditCommand::MoveLeft,
        "moveright" => EditCommand::MoveRight,
        "movewordleft" => EditCommand::MoveWordLeft,
        "movebigwordleft" => EditCommand::MoveBigWordLeft,
        "movewordright" => EditCommand::MoveWordRight,
        "movewordrightend" => EditCommand::MoveWordRightEnd,
        "movebigwordrightend" => EditCommand::MoveBigWordRightEnd,
        "movewordrightstart" => EditCommand::MoveWordRightStart,
        "movebigwordrightstart" => EditCommand::MoveBigWordRightStart,
        "movetoposition" => {
            let value = extract_value("value", cols, vals, span)?;
            EditCommand::MoveToPosition(value.as_integer()? as usize)
        }
        "insertchar" => {
            let value = extract_value("value", cols, vals, span)?;
            let char = extract_char(value, config)?;
            EditCommand::InsertChar(char)
        }
        "insertstring" => {
            let value = extract_value("value", cols, vals, span)?;
            EditCommand::InsertString(value.into_string("", config))
        }
        "insertnewline" => EditCommand::InsertNewline,
        "backspace" => EditCommand::Backspace,
        "delete" => EditCommand::Delete,
        "cutchar" => EditCommand::CutChar,
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
        "cutbigwordleft" => EditCommand::CutBigWordLeft,
        "cutwordright" => EditCommand::CutWordRight,
        "cutbigwordright" => EditCommand::CutBigWordRight,
        "cutwordrighttonext" => EditCommand::CutWordRightToNext,
        "cutbigwordrighttonext" => EditCommand::CutBigWordRightToNext,
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
            let value = extract_value("value", cols, vals, span)?;
            let char = extract_char(value, config)?;
            EditCommand::CutRightUntil(char)
        }
        "cutrightbefore" => {
            let value = extract_value("value", cols, vals, span)?;
            let char = extract_char(value, config)?;
            EditCommand::CutRightBefore(char)
        }
        "moverightuntil" => {
            let value = extract_value("value", cols, vals, span)?;
            let char = extract_char(value, config)?;
            EditCommand::MoveRightUntil(char)
        }
        "moverightbefore" => {
            let value = extract_value("value", cols, vals, span)?;
            let char = extract_char(value, config)?;
            EditCommand::MoveRightBefore(char)
        }
        "cutleftuntil" => {
            let value = extract_value("value", cols, vals, span)?;
            let char = extract_char(value, config)?;
            EditCommand::CutLeftUntil(char)
        }
        "cutleftbefore" => {
            let value = extract_value("value", cols, vals, span)?;
            let char = extract_char(value, config)?;
            EditCommand::CutLeftBefore(char)
        }
        "moveleftuntil" => {
            let value = extract_value("value", cols, vals, span)?;
            let char = extract_char(value, config)?;
            EditCommand::MoveLeftUntil(char)
        }
        "moveleftbefore" => {
            let value = extract_value("value", cols, vals, span)?;
            let char = extract_char(value, config)?;
            EditCommand::MoveLeftBefore(char)
        }
        "complete" => EditCommand::Complete,
        e => {
            return Err(ShellError::UnsupportedConfigValue(
                "reedline EditCommand".to_string(),
                e.to_string(),
                *span,
            ))
        }
    };

    Ok(edit)
}

fn extract_char(value: &Value, config: &Config) -> Result<char, ShellError> {
    let span = value.span()?;
    value
        .into_string("", config)
        .chars()
        .next()
        .ok_or_else(|| ShellError::MissingConfigValue("char to insert".to_string(), span))
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
        assert_eq!(parsed_event, Some(ReedlineEvent::Enter));
    }

    #[test]
    fn test_edit_event() {
        let cols = vec!["edit".to_string()];
        let vals = vec![Value::String {
            val: "Clear".to_string(),
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
        assert_eq!(
            parsed_event,
            Some(ReedlineEvent::Edit(vec![EditCommand::Clear]))
        );
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
            Some(ReedlineEvent::Menu("history_menu".to_string()))
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
            Some(ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("history_menu".to_string()),
                ReedlineEvent::Enter,
            ]))
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
            Some(ReedlineEvent::Multiple(vec![
                ReedlineEvent::Menu("history_menu".to_string()),
                ReedlineEvent::Enter,
            ]))
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
