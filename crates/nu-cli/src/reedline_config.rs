use crate::{NuHelpCompleter, menus::NuMenuCompleter};
use crossterm::event::{KeyCode, KeyModifiers};
use nu_ansi_term::Style;
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    Config, EditBindings, FromValue, ParsedKeybinding, ParsedMenu, PipelineData, Record,
    ShellError, Span, Type, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    extract_value,
};
use reedline::{
    ColumnarMenu, DescriptionMenu, DescriptionMode, EditCommand, IdeMenu, Keybindings, ListMenu,
    MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, default_emacs_keybindings,
    default_vi_insert_keybindings, default_vi_normal_keybindings,
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

const DEFAULT_IDE_COMPLETION_MENU: &str = r#"
{
  name: ide_completion_menu
  only_buffer_difference: false
  marker: "| "
  type: {
    layout: ide
    min_completion_width: 0,
    max_completion_width: 50,
    max_completion_height: 10, # will be limited by the available lines in the terminal
    padding: 0,
    border: true,
    cursor_offset: 0,
    description_mode: "prefer_right"
    min_description_width: 0
    max_description_width: 50
    max_description_height: 10
    description_offset: 1
    # If true, the cursor pos will be corrected, so the suggestions match up with the typed text
    #
    # C:\> str
    #      str join
    #      str trim
    #      str split
    correct_cursor_pos: false
  }
  style: {
    text: green
    selected_text: { attr: r }
    description_text: yellow
    match_text: { attr: u }
    selected_match_text: { attr: ur }
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
    engine_state_ref: Arc<EngineState>,
    stack: &Stack,
    config: Arc<Config>,
) -> Result<Reedline, ShellError> {
    //log::trace!("add_menus: config: {:#?}", &config);
    line_editor = line_editor.clear_menus();

    for menu in &config.menus {
        line_editor = add_menu(
            line_editor,
            menu,
            engine_state_ref.clone(),
            stack,
            config.clone(),
        )?
    }

    // Checking if the default menus have been added from the config file
    let default_menus = [
        ("completion_menu", DEFAULT_COMPLETION_MENU),
        ("ide_completion_menu", DEFAULT_IDE_COMPLETION_MENU),
        ("history_menu", DEFAULT_HISTORY_MENU),
        ("help_menu", DEFAULT_HELP_MENU),
    ];

    let mut engine_state = (*engine_state_ref).clone();
    let mut menu_eval_results = vec![];

    for (name, definition) in default_menus {
        if !config
            .menus
            .iter()
            .any(|menu| menu.name.to_expanded_string("", &config) == name)
        {
            let (block, delta) = {
                let mut working_set = StateWorkingSet::new(&engine_state);
                let output = parse(
                    &mut working_set,
                    Some(name), // format!("entry #{}", entry_num)
                    definition.as_bytes(),
                    true,
                );

                (output, working_set.render())
            };

            engine_state.merge_delta(delta)?;

            let mut temp_stack = Stack::new().collect_value();
            let input = PipelineData::empty();
            menu_eval_results.push(eval_block::<WithoutDebug>(
                &engine_state,
                &mut temp_stack,
                &block,
                input,
            )?);
        }
    }

    let new_engine_state_ref = Arc::new(engine_state);

    for res in menu_eval_results.into_iter() {
        if let PipelineData::Value(value, None) = res {
            line_editor = add_menu(
                line_editor,
                &ParsedMenu::from_value(value)?,
                new_engine_state_ref.clone(),
                stack,
                config.clone(),
            )?;
        }
    }

    Ok(line_editor)
}

fn add_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: Arc<Config>,
) -> Result<Reedline, ShellError> {
    let span = menu.r#type.span();
    if let Value::Record { val, .. } = &menu.r#type {
        let layout = extract_value("layout", val, span)?.to_expanded_string("", &config);

        match layout.as_str() {
            "columnar" => add_columnar_menu(line_editor, menu, engine_state, stack, &config),
            "list" => add_list_menu(line_editor, menu, engine_state, stack, config),
            "ide" => add_ide_menu(line_editor, menu, engine_state, stack, config),
            "description" => add_description_menu(line_editor, menu, engine_state, stack, config),
            str => Err(ShellError::InvalidValue {
                valid: "'columnar', 'list', 'ide', or 'description'".into(),
                actual: format!("'{str}'"),
                span,
            }),
        }
    } else {
        Err(ShellError::RuntimeTypeMismatch {
            expected: Type::record(),
            actual: menu.r#type.get_type(),
            span,
        })
    }
}

fn get_style(record: &Record, name: &'static str, span: Span) -> Option<Style> {
    extract_value(name, record, span)
        .ok()
        .map(|text| match text {
            Value::String { val, .. } => lookup_ansi_color_style(val),
            Value::Record { .. } => color_record_to_nustyle(text),
            _ => lookup_ansi_color_style("green"),
        })
}

fn set_menu_style<M: MenuBuilder>(mut menu: M, style: &Value) -> M {
    let span = style.span();
    let Value::Record { val, .. } = &style else {
        return menu;
    };
    if let Some(style) = get_style(val, "text", span) {
        menu = menu.with_text_style(style);
    }
    if let Some(style) = get_style(val, "selected_text", span) {
        menu = menu.with_selected_text_style(style);
    }
    if let Some(style) = get_style(val, "description_text", span) {
        menu = menu.with_description_text_style(style);
    }
    if let Some(style) = get_style(val, "match_text", span) {
        menu = menu.with_match_text_style(style);
    }
    if let Some(style) = get_style(val, "selected_match_text", span) {
        menu = menu.with_selected_match_text_style(style);
    }
    menu
}

// Adds a columnar menu to the editor engine
pub(crate) fn add_columnar_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: &Config,
) -> Result<Reedline, ShellError> {
    let span = menu.r#type.span();
    let name = menu.name.to_expanded_string("", config);
    let mut columnar_menu = ColumnarMenu::default().with_name(&name);

    if let Value::Record { val, .. } = &menu.r#type {
        columnar_menu = match extract_value("columns", val, span) {
            Ok(columns) => {
                let columns = columns.as_int()?;
                columnar_menu.with_columns(columns as u16)
            }
            Err(_) => columnar_menu,
        };

        columnar_menu = match extract_value("col_width", val, span) {
            Ok(col_width) => {
                let col_width = col_width.as_int()?;
                columnar_menu.with_column_width(Some(col_width as usize))
            }
            Err(_) => columnar_menu.with_column_width(None),
        };

        columnar_menu = match extract_value("col_padding", val, span) {
            Ok(col_padding) => {
                let col_padding = col_padding.as_int()?;
                columnar_menu.with_column_padding(col_padding as usize)
            }
            Err(_) => columnar_menu,
        };
    }

    columnar_menu = set_menu_style(columnar_menu, &menu.style);

    let marker = menu.marker.to_expanded_string("", config);
    columnar_menu = columnar_menu.with_marker(&marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    columnar_menu = columnar_menu.with_only_buffer_difference(only_buffer_difference);

    let completer = if let Some(closure) = &menu.source {
        let menu_completer = NuMenuCompleter::new(
            closure.block_id,
            span,
            stack.captures_to_stack(closure.captures.clone()),
            engine_state,
            only_buffer_difference,
        );
        ReedlineMenu::WithCompleter {
            menu: Box::new(columnar_menu),
            completer: Box::new(menu_completer),
        }
    } else {
        ReedlineMenu::EngineCompleter(Box::new(columnar_menu))
    };

    Ok(line_editor.with_menu(completer))
}

// Adds a search menu to the line editor
pub(crate) fn add_list_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: Arc<Config>,
) -> Result<Reedline, ShellError> {
    let name = menu.name.to_expanded_string("", &config);
    let mut list_menu = ListMenu::default().with_name(&name);

    let span = menu.r#type.span();
    if let Value::Record { val, .. } = &menu.r#type {
        list_menu = match extract_value("page_size", val, span) {
            Ok(page_size) => {
                let page_size = page_size.as_int()?;
                list_menu.with_page_size(page_size as usize)
            }
            Err(_) => list_menu,
        };
    }

    list_menu = set_menu_style(list_menu, &menu.style);

    let marker = menu.marker.to_expanded_string("", &config);
    list_menu = list_menu.with_marker(&marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    list_menu = list_menu.with_only_buffer_difference(only_buffer_difference);

    let completer = if let Some(closure) = &menu.source {
        let menu_completer = NuMenuCompleter::new(
            closure.block_id,
            span,
            stack.captures_to_stack(closure.captures.clone()),
            engine_state,
            only_buffer_difference,
        );
        ReedlineMenu::WithCompleter {
            menu: Box::new(list_menu),
            completer: Box::new(menu_completer),
        }
    } else {
        ReedlineMenu::HistoryMenu(Box::new(list_menu))
    };

    Ok(line_editor.with_menu(completer))
}

// Adds an IDE menu to the line editor
pub(crate) fn add_ide_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: Arc<Config>,
) -> Result<Reedline, ShellError> {
    let span = menu.r#type.span();
    let name = menu.name.to_expanded_string("", &config);
    let mut ide_menu = IdeMenu::default().with_name(&name);

    if let Value::Record { val, .. } = &menu.r#type {
        ide_menu = match extract_value("min_completion_width", val, span) {
            Ok(min_completion_width) => {
                let min_completion_width = min_completion_width.as_int()?;
                ide_menu.with_min_completion_width(min_completion_width as u16)
            }
            Err(_) => ide_menu,
        };

        ide_menu = match extract_value("max_completion_width", val, span) {
            Ok(max_completion_width) => {
                let max_completion_width = max_completion_width.as_int()?;
                ide_menu.with_max_completion_width(max_completion_width as u16)
            }
            Err(_) => ide_menu,
        };

        ide_menu = match extract_value("max_completion_height", val, span) {
            Ok(max_completion_height) => {
                let max_completion_height = max_completion_height.as_int()?;
                ide_menu.with_max_completion_height(max_completion_height as u16)
            }
            Err(_) => ide_menu.with_max_completion_height(10u16),
        };

        ide_menu = match extract_value("padding", val, span) {
            Ok(padding) => {
                let padding = padding.as_int()?;
                ide_menu.with_padding(padding as u16)
            }
            Err(_) => ide_menu,
        };

        ide_menu = match extract_value("border", val, span) {
            Ok(border) => {
                if let Ok(border) = border.as_bool() {
                    if border {
                        ide_menu.with_default_border()
                    } else {
                        ide_menu
                    }
                } else if let Ok(border_chars) = border.as_record() {
                    let top_right = extract_value("top_right", border_chars, span)?.as_char()?;
                    let top_left = extract_value("top_left", border_chars, span)?.as_char()?;
                    let bottom_right =
                        extract_value("bottom_right", border_chars, span)?.as_char()?;
                    let bottom_left =
                        extract_value("bottom_left", border_chars, span)?.as_char()?;
                    let horizontal = extract_value("horizontal", border_chars, span)?.as_char()?;
                    let vertical = extract_value("vertical", border_chars, span)?.as_char()?;

                    ide_menu.with_border(
                        top_right,
                        top_left,
                        bottom_right,
                        bottom_left,
                        horizontal,
                        vertical,
                    )
                } else {
                    return Err(ShellError::RuntimeTypeMismatch {
                        expected: Type::custom("bool or record"),
                        actual: border.get_type(),
                        span: border.span(),
                    });
                }
            }
            Err(_) => ide_menu.with_default_border(),
        };

        ide_menu = match extract_value("cursor_offset", val, span) {
            Ok(cursor_offset) => {
                let cursor_offset = cursor_offset.as_int()?;
                ide_menu.with_cursor_offset(cursor_offset as i16)
            }
            Err(_) => ide_menu,
        };

        ide_menu = match extract_value("description_mode", val, span) {
            Ok(description_mode) => match description_mode.coerce_str()?.as_ref() {
                "left" => ide_menu.with_description_mode(DescriptionMode::Left),
                "right" => ide_menu.with_description_mode(DescriptionMode::Right),
                "prefer_right" => ide_menu.with_description_mode(DescriptionMode::PreferRight),
                str => {
                    return Err(ShellError::InvalidValue {
                        valid: "'left', 'right', or 'prefer_right'".into(),
                        actual: format!("'{str}'"),
                        span: description_mode.span(),
                    });
                }
            },
            Err(_) => ide_menu,
        };

        ide_menu = match extract_value("min_description_width", val, span) {
            Ok(min_description_width) => {
                let min_description_width = min_description_width.as_int()?;
                ide_menu.with_min_description_width(min_description_width as u16)
            }
            Err(_) => ide_menu,
        };

        ide_menu = match extract_value("max_description_width", val, span) {
            Ok(max_description_width) => {
                let max_description_width = max_description_width.as_int()?;
                ide_menu.with_max_description_width(max_description_width as u16)
            }
            Err(_) => ide_menu,
        };

        ide_menu = match extract_value("max_description_height", val, span) {
            Ok(max_description_height) => {
                let max_description_height = max_description_height.as_int()?;
                ide_menu.with_max_description_height(max_description_height as u16)
            }
            Err(_) => ide_menu,
        };

        ide_menu = match extract_value("description_offset", val, span) {
            Ok(description_padding) => {
                let description_padding = description_padding.as_int()?;
                ide_menu.with_description_offset(description_padding as u16)
            }
            Err(_) => ide_menu,
        };

        ide_menu = match extract_value("correct_cursor_pos", val, span) {
            Ok(correct_cursor_pos) => {
                let correct_cursor_pos = correct_cursor_pos.as_bool()?;
                ide_menu.with_correct_cursor_pos(correct_cursor_pos)
            }
            Err(_) => ide_menu,
        };
    }

    ide_menu = set_menu_style(ide_menu, &menu.style);

    let marker = menu.marker.to_expanded_string("", &config);
    ide_menu = ide_menu.with_marker(&marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    ide_menu = ide_menu.with_only_buffer_difference(only_buffer_difference);

    let completer = if let Some(closure) = &menu.source {
        let menu_completer = NuMenuCompleter::new(
            closure.block_id,
            span,
            stack.captures_to_stack(closure.captures.clone()),
            engine_state,
            only_buffer_difference,
        );
        ReedlineMenu::WithCompleter {
            menu: Box::new(ide_menu),
            completer: Box::new(menu_completer),
        }
    } else {
        ReedlineMenu::EngineCompleter(Box::new(ide_menu))
    };

    Ok(line_editor.with_menu(completer))
}

// Adds a description menu to the line editor
pub(crate) fn add_description_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: Arc<Config>,
) -> Result<Reedline, ShellError> {
    let name = menu.name.to_expanded_string("", &config);
    let mut description_menu = DescriptionMenu::default().with_name(&name);

    let span = menu.r#type.span();
    if let Value::Record { val, .. } = &menu.r#type {
        description_menu = match extract_value("columns", val, span) {
            Ok(columns) => {
                let columns = columns.as_int()?;
                description_menu.with_columns(columns as u16)
            }
            Err(_) => description_menu,
        };

        description_menu = match extract_value("col_width", val, span) {
            Ok(col_width) => {
                let col_width = col_width.as_int()?;
                description_menu.with_column_width(Some(col_width as usize))
            }
            Err(_) => description_menu.with_column_width(None),
        };

        description_menu = match extract_value("col_padding", val, span) {
            Ok(col_padding) => {
                let col_padding = col_padding.as_int()?;
                description_menu.with_column_padding(col_padding as usize)
            }
            Err(_) => description_menu,
        };

        description_menu = match extract_value("selection_rows", val, span) {
            Ok(selection_rows) => {
                let selection_rows = selection_rows.as_int()?;
                description_menu.with_selection_rows(selection_rows as u16)
            }
            Err(_) => description_menu,
        };

        description_menu = match extract_value("description_rows", val, span) {
            Ok(description_rows) => {
                let description_rows = description_rows.as_int()?;
                description_menu.with_description_rows(description_rows as usize)
            }
            Err(_) => description_menu,
        };
    }

    description_menu = set_menu_style(description_menu, &menu.style);

    let marker = menu.marker.to_expanded_string("", &config);
    description_menu = description_menu.with_marker(&marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    description_menu = description_menu.with_only_buffer_difference(only_buffer_difference);

    let completer = if let Some(closure) = &menu.source {
        let menu_completer = NuMenuCompleter::new(
            closure.block_id,
            span,
            stack.captures_to_stack(closure.captures.clone()),
            engine_state,
            only_buffer_difference,
        );
        ReedlineMenu::WithCompleter {
            menu: Box::new(description_menu),
            completer: Box::new(menu_completer),
        }
    } else {
        let menu_completer = NuHelpCompleter::new(engine_state, config);
        ReedlineMenu::WithCompleter {
            menu: Box::new(description_menu),
            completer: Box::new(menu_completer),
        }
    };

    Ok(line_editor.with_menu(completer))
}

fn add_menu_keybindings(keybindings: &mut Keybindings) {
    // Completer menu keybindings
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
            ReedlineEvent::Edit(vec![EditCommand::Complete]),
        ]),
    );

    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char(' '),
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("ide_completion_menu".to_string()),
            ReedlineEvent::MenuNext,
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

    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('q'),
        ReedlineEvent::SearchHistory,
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

    match config.edit_mode {
        EditBindings::Emacs => {
            add_menu_keybindings(&mut emacs_keybindings);
        }
        EditBindings::Vi => {
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

    match config.edit_mode {
        EditBindings::Emacs => Ok(KeybindingsMode::Emacs(emacs_keybindings)),
        EditBindings::Vi => Ok(KeybindingsMode::Vi {
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
    let span = mode.span();
    match &mode {
        Value::String { val, .. } => match val.as_str() {
            str if str.eq_ignore_ascii_case("emacs") => {
                add_parsed_keybinding(emacs_keybindings, keybinding, config)
            }
            str if str.eq_ignore_ascii_case("vi_insert") => {
                add_parsed_keybinding(insert_keybindings, keybinding, config)
            }
            str if str.eq_ignore_ascii_case("vi_normal") => {
                add_parsed_keybinding(normal_keybindings, keybinding, config)
            }
            str => Err(ShellError::InvalidValue {
                valid: "'emacs', 'vi_insert', or 'vi_normal'".into(),
                actual: format!("'{str}'"),
                span,
            }),
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
        v => Err(ShellError::RuntimeTypeMismatch {
            expected: Type::custom("string or list<string>"),
            actual: v.get_type(),
            span: v.span(),
        }),
    }
}

fn add_parsed_keybinding(
    keybindings: &mut Keybindings,
    keybinding: &ParsedKeybinding,
    config: &Config,
) -> Result<(), ShellError> {
    let Ok(modifier_str) = keybinding.modifier.as_str() else {
        return Err(ShellError::RuntimeTypeMismatch {
            expected: Type::String,
            actual: keybinding.modifier.get_type(),
            span: keybinding.modifier.span(),
        });
    };

    let mut modifier = KeyModifiers::NONE;
    if !str::eq_ignore_ascii_case(modifier_str, "none") {
        for part in modifier_str.split('_') {
            match part.to_ascii_lowercase().as_str() {
                "control" => modifier |= KeyModifiers::CONTROL,
                "shift" => modifier |= KeyModifiers::SHIFT,
                "alt" => modifier |= KeyModifiers::ALT,
                "super" => modifier |= KeyModifiers::SUPER,
                "hyper" => modifier |= KeyModifiers::HYPER,
                "meta" => modifier |= KeyModifiers::META,
                _ => {
                    return Err(ShellError::InvalidValue {
                        valid: "'control', 'shift', 'alt', 'super', 'hyper', 'meta', or 'none'"
                            .into(),
                        actual: format!("'{part}'"),
                        span: keybinding.modifier.span(),
                    });
                }
            }
        }
    }

    let Ok(keycode) = keybinding.keycode.as_str() else {
        return Err(ShellError::RuntimeTypeMismatch {
            expected: Type::String,
            actual: keybinding.keycode.get_type(),
            span: keybinding.keycode.span(),
        });
    };

    let keycode_lower = keycode.to_ascii_lowercase();

    let keycode = if let Some(rest) = keycode_lower.strip_prefix("char_") {
        let error = |valid: &str, actual: &str| ShellError::InvalidValue {
            valid: valid.into(),
            actual: actual.into(),
            span: keybinding.keycode.span(),
        };

        let mut char_iter = rest.chars();
        let char = match (char_iter.next(), char_iter.next()) {
            (Some(char), None) => char,
            (Some('u'), Some(_)) => {
                // This will never panic as we know there are at least two symbols
                let Ok(code_point) = u32::from_str_radix(&rest[1..], 16) else {
                    return Err(error("a valid hex code", keycode));
                };

                char::from_u32(code_point).ok_or(error("a valid Unicode code point", keycode))?
            }
            _ => return Err(error("'char_<char>' or 'char_u<hex code>'", keycode)),
        };

        KeyCode::Char(char)
    } else {
        match keycode_lower.as_str() {
            "backspace" => KeyCode::Backspace,
            "enter" => KeyCode::Enter,
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
            c if c.starts_with('f') => c[1..]
                .parse()
                .ok()
                .filter(|num| (1..=35).contains(num))
                .map(KeyCode::F)
                .ok_or(ShellError::InvalidValue {
                    valid: "'f1', 'f2', ..., or 'f35'".into(),
                    actual: format!("'{keycode}'"),
                    span: keybinding.keycode.span(),
                })?,
            "null" => KeyCode::Null,
            "esc" | "escape" => KeyCode::Esc,
            _ => {
                return Err(ShellError::InvalidValue {
                    valid: "a crossterm KeyCode".into(),
                    actual: format!("'{keycode}'"),
                    span: keybinding.keycode.span(),
                });
            }
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
    fn try_from_record(record: &'config Record, span: Span) -> Result<Self, ShellError> {
        extract_value("send", record, span)
            .map(Self::Send)
            .or_else(|_| extract_value("edit", record, span).map(Self::Edit))
            .or_else(|_| extract_value("until", record, span).map(Self::Until))
            .map_err(|_| ShellError::MissingRequiredColumn {
                column: "'send', 'edit', or 'until'",
                span,
            })
    }
}

fn parse_event(value: &Value, config: &Config) -> Result<Option<ReedlineEvent>, ShellError> {
    let span = value.span();
    match value {
        Value::Record { val: record, .. } => match EventType::try_from_record(record, span)? {
            EventType::Send(value) => event_from_record(
                value
                    .to_expanded_string("", config)
                    .to_ascii_lowercase()
                    .as_str(),
                record,
                config,
                span,
            )
            .map(Some),
            EventType::Edit(value) => {
                let edit = edit_from_record(
                    value
                        .to_expanded_string("", config)
                        .to_ascii_lowercase()
                        .as_str(),
                    record,
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
                                None => Err(ShellError::RuntimeTypeMismatch {
                                    expected: Type::custom("record or table"),
                                    actual: value.get_type(),
                                    span: value.span(),
                                }),
                                Some(event) => Ok(event),
                            },
                            Err(e) => Err(e),
                        })
                        .collect::<Result<Vec<ReedlineEvent>, ShellError>>()?;

                    Ok(Some(ReedlineEvent::UntilFound(events)))
                }
                v => Err(ShellError::RuntimeTypeMismatch {
                    expected: Type::list(Type::Any),
                    actual: v.get_type(),
                    span: v.span(),
                }),
            },
        },
        Value::List { vals, .. } => {
            let events = vals
                .iter()
                .map(|value| match parse_event(value, config) {
                    Ok(inner) => match inner {
                        None => Err(ShellError::RuntimeTypeMismatch {
                            expected: Type::custom("record or table"),
                            actual: value.get_type(),
                            span: value.span(),
                        }),
                        Some(event) => Ok(event),
                    },
                    Err(e) => Err(e),
                })
                .collect::<Result<Vec<ReedlineEvent>, ShellError>>()?;

            Ok(Some(ReedlineEvent::Multiple(events)))
        }
        Value::Nothing { .. } => Ok(None),
        v => Err(ShellError::RuntimeTypeMismatch {
            expected: Type::custom("record, table, or nothing"),
            actual: v.get_type(),
            span: v.span(),
        }),
    }
}

fn event_from_record(
    name: &str,
    record: &Record,
    config: &Config,
    span: Span,
) -> Result<ReedlineEvent, ShellError> {
    let event = match name {
        "none" => ReedlineEvent::None,
        "historyhintcomplete" => ReedlineEvent::HistoryHintComplete,
        "historyhintwordcomplete" => ReedlineEvent::HistoryHintWordComplete,
        "ctrld" => ReedlineEvent::CtrlD,
        "ctrlc" => ReedlineEvent::CtrlC,
        "clearscreen" => ReedlineEvent::ClearScreen,
        "clearscrollback" => ReedlineEvent::ClearScrollback,
        "enter" => ReedlineEvent::Enter,
        "submit" => ReedlineEvent::Submit,
        "submitornewline" => ReedlineEvent::SubmitOrNewline,
        "esc" | "escape" => ReedlineEvent::Esc,
        // Non-sensical for user configuration:
        //
        // `ReedlineEvent::Mouse` - itself a no-op
        // `ReedlineEvent::Resize` - requires size info specifically from the ANSI resize
        // event
        //
        // Handled above in `parse_event`:
        //
        // `ReedlineEvent::Edit`
        "repaint" => ReedlineEvent::Repaint,
        "previoushistory" => ReedlineEvent::PreviousHistory,
        "up" => ReedlineEvent::Up,
        "down" => ReedlineEvent::Down,
        "right" => ReedlineEvent::Right,
        "left" => ReedlineEvent::Left,
        "nexthistory" => ReedlineEvent::NextHistory,
        "searchhistory" => ReedlineEvent::SearchHistory,
        // Handled above in `parse_event`:
        //
        // `ReedlineEvent::Multiple`
        // `ReedlineEvent::UntilFound`
        "menu" => {
            let menu = extract_value("name", record, span)?;
            ReedlineEvent::Menu(menu.to_expanded_string("", config))
        }
        "menunext" => ReedlineEvent::MenuNext,
        "menuprevious" => ReedlineEvent::MenuPrevious,
        "menuup" => ReedlineEvent::MenuUp,
        "menudown" => ReedlineEvent::MenuDown,
        "menuleft" => ReedlineEvent::MenuLeft,
        "menuright" => ReedlineEvent::MenuRight,
        "menupagenext" => ReedlineEvent::MenuPageNext,
        "menupageprevious" => ReedlineEvent::MenuPagePrevious,
        "executehostcommand" => {
            let cmd = extract_value("cmd", record, span)?;
            ReedlineEvent::ExecuteHostCommand(cmd.to_expanded_string("", config))
        }
        "openeditor" => ReedlineEvent::OpenEditor,
        "vichangemode" => {
            let mode = extract_value("mode", record, span)?;
            ReedlineEvent::ViChangeMode(mode.as_str()?.to_owned())
        }
        str => {
            return Err(ShellError::InvalidValue {
                valid: "a reedline event".into(),
                actual: format!("'{str}'"),
                span,
            });
        }
    };

    Ok(event)
}

fn edit_from_record(
    name: &str,
    record: &Record,
    config: &Config,
    span: Span,
) -> Result<EditCommand, ShellError> {
    let edit = match name {
        "movetostart" => EditCommand::MoveToStart {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movetolinestart" => EditCommand::MoveToLineStart {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movetoend" => EditCommand::MoveToEnd {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movetolineend" => EditCommand::MoveToLineEnd {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "moveleft" => EditCommand::MoveLeft {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "moveright" => EditCommand::MoveRight {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movewordleft" => EditCommand::MoveWordLeft {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movebigwordleft" => EditCommand::MoveBigWordLeft {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movewordright" => EditCommand::MoveWordRight {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movewordrightstart" => EditCommand::MoveWordRightStart {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movebigwordrightstart" => EditCommand::MoveBigWordRightStart {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movewordrightend" => EditCommand::MoveWordRightEnd {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movebigwordrightend" => EditCommand::MoveBigWordRightEnd {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        "movetoposition" => {
            let value = extract_value("value", record, span)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            EditCommand::MoveToPosition {
                position: value.as_int()? as usize,
                select,
            }
        }
        "insertchar" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::InsertChar(char)
        }
        "insertstring" => {
            let value = extract_value("value", record, span)?;
            EditCommand::InsertString(value.to_expanded_string("", config))
        }
        "insertnewline" => EditCommand::InsertNewline,
        "replacechar" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::ReplaceChar(char)
        }
        // `EditCommand::ReplaceChars` - Internal hack not sanely implementable as a
        // standalone binding
        "backspace" => EditCommand::Backspace,
        "delete" => EditCommand::Delete,
        "cutchar" => EditCommand::CutChar,
        "backspaceword" => EditCommand::BackspaceWord,
        "deleteword" => EditCommand::DeleteWord,
        "clear" => EditCommand::Clear,
        "cleartolineend" => EditCommand::ClearToLineEnd,
        "complete" => EditCommand::Complete,
        "cutcurrentline" => EditCommand::CutCurrentLine,
        "cutfromstart" => EditCommand::CutFromStart,
        "cutfromlinestart" => EditCommand::CutFromLineStart,
        "cuttoend" => EditCommand::CutToEnd,
        "cuttolineend" => EditCommand::CutToLineEnd,
        "killline" => EditCommand::KillLine,
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
        "switchcasechar" => EditCommand::SwitchcaseChar,
        "swapwords" => EditCommand::SwapWords,
        "swapgraphemes" => EditCommand::SwapGraphemes,
        "undo" => EditCommand::Undo,
        "redo" => EditCommand::Redo,
        "cutrightuntil" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CutRightUntil(char)
        }
        "cutrightbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CutRightBefore(char)
        }
        "moverightuntil" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveRightUntil { c: char, select }
        }
        "moverightbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveRightBefore { c: char, select }
        }
        "cutleftuntil" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CutLeftUntil(char)
        }
        "cutleftbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CutLeftBefore(char)
        }
        "moveleftuntil" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveLeftUntil { c: char, select }
        }
        "moveleftbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveLeftBefore { c: char, select }
        }
        "selectall" => EditCommand::SelectAll,
        "cutselection" => EditCommand::CutSelection,
        "copyselection" => EditCommand::CopySelection,
        "paste" => EditCommand::Paste,
        "copyfromstart" => EditCommand::CopyFromStart,
        "copyfromlinestart" => EditCommand::CopyFromLineStart,
        "copytoend" => EditCommand::CopyToEnd,
        "copytolineend" => EditCommand::CopyToLineEnd,
        "copycurrentline" => EditCommand::CopyCurrentLine,
        "copywordleft" => EditCommand::CopyWordLeft,
        "copybigwordleft" => EditCommand::CopyBigWordLeft,
        "copywordright" => EditCommand::CopyWordRight,
        "copybigwordright" => EditCommand::CopyBigWordRight,
        "copywordrighttonext" => EditCommand::CopyWordRightToNext,
        "copybigwordrighttonext" => EditCommand::CopyBigWordRightToNext,
        "copyleft" => EditCommand::CopyLeft,
        "copyright" => EditCommand::CopyRight,
        "copyrightuntil" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CopyRightUntil(char)
        }
        "copyrightbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CopyRightBefore(char)
        }
        "copyleftuntil" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CopyLeftUntil(char)
        }
        "copyleftbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CopyLeftBefore(char)
        }
        "swapcursorandanchor" => EditCommand::SwapCursorAndAnchor,
        #[cfg(feature = "system-clipboard")]
        "cutselectionsystem" => EditCommand::CutSelectionSystem,
        #[cfg(feature = "system-clipboard")]
        "copyselectionsystem" => EditCommand::CopySelectionSystem,
        #[cfg(feature = "system-clipboard")]
        "pastesystem" => EditCommand::PasteSystem,
        "cutinside" => {
            let value = extract_value("left", record, span)?;
            let left = extract_char(value)?;
            let value = extract_value("right", record, span)?;
            let right = extract_char(value)?;
            EditCommand::CutInside { left, right }
        }
        "yankinside" => {
            let value = extract_value("left", record, span)?;
            let left = extract_char(value)?;
            let value = extract_value("right", record, span)?;
            let right = extract_char(value)?;
            EditCommand::YankInside { left, right }
        }
        str => {
            return Err(ShellError::InvalidValue {
                valid: "a reedline EditCommand".into(),
                actual: format!("'{str}'"),
                span,
            });
        }
    };

    Ok(edit)
}

fn extract_char(value: &Value) -> Result<char, ShellError> {
    if let Ok(str) = value.as_str() {
        let mut chars = str.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(ShellError::InvalidValue {
                valid: "a single character".into(),
                actual: format!("'{str}'"),
                span: value.span(),
            }),
        }
    } else {
        Err(ShellError::RuntimeTypeMismatch {
            expected: Type::String,
            actual: value.get_type(),
            span: value.span(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::record;

    #[test]
    fn test_send_event() {
        let event = record! {
            "send" => Value::test_string("Enter"),
        };

        let span = Span::test_data();
        let b = EventType::try_from_record(&event, span).unwrap();
        assert!(matches!(b, EventType::Send(_)));

        let event = Value::test_record(event);
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(parsed_event, Some(ReedlineEvent::Enter));
    }

    #[test]
    fn test_edit_event() {
        let event = record! {
            "edit" => Value::test_string("Clear"),
        };

        let span = Span::test_data();
        let b = EventType::try_from_record(&event, span).unwrap();
        assert!(matches!(b, EventType::Edit(_)));

        let event = Value::test_record(event);
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            Some(ReedlineEvent::Edit(vec![EditCommand::Clear]))
        );
    }

    #[test]
    fn test_send_menu() {
        let event = record! {
            "send" =>  Value::test_string("Menu"),
            "name" =>  Value::test_string("history_menu"),
        };

        let span = Span::test_data();
        let b = EventType::try_from_record(&event, span).unwrap();
        assert!(matches!(b, EventType::Send(_)));

        let event = Value::test_record(event);
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            Some(ReedlineEvent::Menu("history_menu".to_string()))
        );
    }

    #[test]
    fn test_until_event() {
        let menu_event = Value::test_record(record! {
            "send" =>  Value::test_string("Menu"),
            "name" =>  Value::test_string("history_menu"),
        });
        let enter_event = Value::test_record(record! {
            "send" => Value::test_string("Enter"),
        });
        let event = record! {
            "until" => Value::list(
                vec![menu_event, enter_event],
                Span::test_data(),
            ),
        };

        let span = Span::test_data();
        let b = EventType::try_from_record(&event, span).unwrap();
        assert!(matches!(b, EventType::Until(_)));

        let event = Value::test_record(event);
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
        let menu_event = Value::test_record(record! {
            "send" => Value::test_string("Menu"),
            "name" => Value::test_string("history_menu"),
        });
        let enter_event = Value::test_record(record! {
            "send" => Value::test_string("Enter"),
        });
        let event = Value::list(vec![menu_event, enter_event], Span::test_data());

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
        let event = record! {
            "not_exist" => Value::test_string("Enter"),
        };

        let span = Span::test_data();
        let b = EventType::try_from_record(&event, span);
        assert!(matches!(b, Err(ShellError::MissingRequiredColumn { .. })));
    }

    #[test]
    fn test_move_without_optional_select() {
        let event = record! {
            "edit" => Value::test_string("moveleft")
        };
        let event = Value::test_record(event);
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            Some(ReedlineEvent::Edit(vec![EditCommand::MoveLeft {
                select: false
            }]))
        );
    }

    #[test]
    fn test_move_with_select_false() {
        let event = record! {
            "edit" => Value::test_string("moveleft"),
            "select" => Value::test_bool(false)
        };
        let event = Value::test_record(event);
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            Some(ReedlineEvent::Edit(vec![EditCommand::MoveLeft {
                select: false
            }]))
        );
    }

    #[test]
    fn test_move_with_select_true() {
        let event = record! {
            "edit" => Value::test_string("moveleft"),
            "select" => Value::test_bool(true)
        };
        let event = Value::test_record(event);
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            Some(ReedlineEvent::Edit(vec![EditCommand::MoveLeft {
                select: true
            }]))
        );
    }
}
