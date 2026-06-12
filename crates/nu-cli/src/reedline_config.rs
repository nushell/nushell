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
    ColumnarMenu, DescriptionMenu, DescriptionMode, DescriptionPosition, Direction, EditCommand,
    EditCommandDiscriminants, FindStop, Granularity, IdeMenu, InputMode, Keybindings, ListMenu,
    MenuBuilder, MotionTarget, OutputMode, PromptEditModeDiscriminants, Reedline, ReedlineEvent,
    ReedlineEventDiscriminants, ReedlineMenu, TextObject, TextObjectScope, TextObjectType,
    TraversalDirection, WordEdge, WordKind, default_emacs_keybindings,
    default_vi_insert_keybindings, default_vi_normal_keybindings,
};
use std::{str::FromStr, sync::Arc};

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
      tab_traversal: "horizontal"
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
    min_description_width: 15
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
      description_rows: 15
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
                    Some(name), // format!("repl_entry #{}", entry_num)
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

    for res in menu_eval_results.into_iter().map(|p| p.body) {
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

fn parse_input_mode(value: &Value, config: &Config) -> Result<InputMode, ShellError> {
    match value
        .to_expanded_string("", config)
        .to_ascii_lowercase()
        .as_str()
    {
        "diff" => Ok(InputMode::Diff),
        "cursor_prefix" => Ok(InputMode::CursorPrefix),
        "full_buffer" => Ok(InputMode::FullBuffer),
        other => Err(ShellError::InvalidValue {
            valid: "'diff', 'cursor_prefix', or 'full_buffer'".into(),
            actual: format!("'{other}'"),
            span: value.span(),
        }),
    }
}

fn parse_output_mode(value: &Value, config: &Config) -> Result<OutputMode, ShellError> {
    match value
        .to_expanded_string("", config)
        .to_ascii_lowercase()
        .as_str()
    {
        "suggested_span" => Ok(OutputMode::SuggestedSpan),
        "full_buffer" => Ok(OutputMode::FullBuffer),
        "extend_to_end" => Ok(OutputMode::ExtendToEnd),
        other => Err(ShellError::InvalidValue {
            valid: "'suggested_span', 'full_buffer', or 'extend_to_end'".into(),
            actual: format!("'{other}'"),
            span: value.span(),
        }),
    }
}

fn parse_description_position(
    value: &Value,
    config: &Config,
) -> Result<DescriptionPosition, ShellError> {
    match value
        .to_expanded_string("", config)
        .to_ascii_lowercase()
        .as_str()
    {
        "before" => Ok(DescriptionPosition::Before),
        "after" => Ok(DescriptionPosition::After),
        other => Err(ShellError::InvalidValue {
            valid: "'before' or 'after'".into(),
            actual: format!("'{other}'"),
            span: value.span(),
        }),
    }
}

/// Resolve the menu's effective reedline `InputMode` from the optional
/// `input_mode` and legacy `only_buffer_difference` fields. The result drives
/// both the reedline menu and `NuMenuCompleter`'s span math, so it must be
/// resolved exactly once, here.
fn resolve_input_mode(menu: &ParsedMenu, config: &Config) -> Result<InputMode, ShellError> {
    match (&menu.input_mode, &menu.only_buffer_difference) {
        (Some(input_mode), _) => parse_input_mode(input_mode, config),
        (None, Some(only_buffer_difference)) => {
            if only_buffer_difference.as_bool()? {
                Ok(InputMode::Diff)
            } else {
                Ok(InputMode::CursorPrefix)
            }
        }
        (None, None) => Err(ShellError::MissingRequiredColumn {
            column: "input_mode (or only_buffer_difference)",
            span: menu.name.span(),
        }),
    }
}

/// Apply the optional reedline #1071 `output_mode`. Unset preserves
/// reedline's default (`suggested_span`).
fn apply_output_mode<M: MenuBuilder>(
    mut menu: M,
    parsed: &ParsedMenu,
    config: &Config,
) -> Result<M, ShellError> {
    if let Some(value) = &parsed.output_mode {
        menu = menu.with_output_mode(parse_output_mode(value, config)?);
    }
    Ok(menu)
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

        columnar_menu = match extract_value("tab_traversal", val, span) {
            Ok(tab_traversal) => match tab_traversal.coerce_str()?.as_ref() {
                "vertical" => columnar_menu.with_traversal_direction(TraversalDirection::Vertical),
                "horizontal" => {
                    columnar_menu.with_traversal_direction(TraversalDirection::Horizontal)
                }
                str => {
                    return Err(ShellError::InvalidValue {
                        valid: "'horizontal' or 'vertical'".into(),
                        actual: format!("'{str}'"),
                        span: tab_traversal.span(),
                    });
                }
            },
            Err(_) => columnar_menu,
        };
    }

    columnar_menu = set_menu_style(columnar_menu, &menu.style);

    let marker = menu.marker.to_expanded_string("", config);
    columnar_menu = columnar_menu.with_marker(&marker);

    let input_mode = resolve_input_mode(menu, config)?;
    columnar_menu = columnar_menu.with_input_mode(input_mode);
    columnar_menu = apply_output_mode(columnar_menu, menu, config)?;

    let completer = if let Some(closure) = &menu.source {
        let menu_completer = NuMenuCompleter::new(
            closure.block_id,
            span,
            stack.captures_to_stack(closure.captures.clone()),
            engine_state,
            input_mode,
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

        list_menu = match extract_value("description_position", val, span) {
            Ok(position) => {
                list_menu.with_description_position(parse_description_position(position, &config)?)
            }
            Err(_) => list_menu,
        };
    }

    list_menu = set_menu_style(list_menu, &menu.style);

    let marker = menu.marker.to_expanded_string("", &config);
    list_menu = list_menu.with_marker(&marker);

    let input_mode = resolve_input_mode(menu, &config)?;
    list_menu = list_menu.with_input_mode(input_mode);
    list_menu = apply_output_mode(list_menu, menu, &config)?;

    let completer = if let Some(closure) = &menu.source {
        let menu_completer = NuMenuCompleter::new(
            closure.block_id,
            span,
            stack.captures_to_stack(closure.captures.clone()),
            engine_state,
            input_mode,
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

    let input_mode = resolve_input_mode(menu, &config)?;
    ide_menu = ide_menu.with_input_mode(input_mode);
    ide_menu = apply_output_mode(ide_menu, menu, &config)?;

    let completer = if let Some(closure) = &menu.source {
        let menu_completer = NuMenuCompleter::new(
            closure.block_id,
            span,
            stack.captures_to_stack(closure.captures.clone()),
            engine_state,
            input_mode,
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

    let input_mode = resolve_input_mode(menu, &config)?;
    description_menu = description_menu.with_input_mode(input_mode);
    description_menu = apply_output_mode(description_menu, menu, &config)?;

    let completer = if let Some(closure) = &menu.source {
        let menu_completer = NuMenuCompleter::new(
            closure.block_id,
            span,
            stack.captures_to_stack(closure.captures.clone()),
            engine_state,
            input_mode,
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
    use PromptEditModeDiscriminants as PEMD;
    let span = mode.span();
    match &mode {
        // When updating this implementation, also update `display_edit_mode` function
        Value::String { val, .. } => match PEMD::from_str(val) {
            Ok(PEMD::Emacs) => add_parsed_keybinding(emacs_keybindings, keybinding, config),
            Ok(PEMD::ViInsert) => add_parsed_keybinding(insert_keybindings, keybinding, config),
            Ok(PEMD::ViNormal) => add_parsed_keybinding(normal_keybindings, keybinding, config),
            Ok(PEMD::Default | PEMD::Custom) | Err(_) => Err(ShellError::InvalidValue {
                valid: "'emacs', 'vi_insert', or 'vi_normal'".into(),
                actual: format!("'{val}'"),
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

// This is displayed in `keybindings list` command
pub(crate) fn display_edit_mode(mode: PromptEditModeDiscriminants) -> Option<String> {
    match mode {
        PromptEditModeDiscriminants::Emacs => Some("emacs".into()),
        PromptEditModeDiscriminants::ViNormal => Some("vi_normal".into()),
        PromptEditModeDiscriminants::ViInsert => Some("vi_insert".into()),
        PromptEditModeDiscriminants::Default | PromptEditModeDiscriminants::Custom => None,
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
                value.to_expanded_string("", config).as_str(),
                record,
                config,
                span,
            )
            .map(Some),
            EventType::Edit(value) => {
                let edit = edit_from_record(
                    value.to_expanded_string("", config).as_str(),
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
    use ReedlineEventDiscriminants as RED;
    // When updating this implementation, also update `display_reedline_event` function
    let event = match RED::from_str(name) {
        Ok(RED::None) => ReedlineEvent::None,
        Ok(RED::HistoryHintComplete) => ReedlineEvent::HistoryHintComplete,
        Ok(RED::HistoryHintWordComplete) => ReedlineEvent::HistoryHintWordComplete,
        Ok(RED::CtrlD) => ReedlineEvent::CtrlD,
        Ok(RED::CtrlC) => ReedlineEvent::CtrlC,
        Ok(RED::ClearScreen) => ReedlineEvent::ClearScreen,
        Ok(RED::ClearScrollback) => ReedlineEvent::ClearScrollback,
        Ok(RED::Enter) => ReedlineEvent::Enter,
        Ok(RED::Submit) => ReedlineEvent::Submit,
        Ok(RED::SubmitOrNewline) => ReedlineEvent::SubmitOrNewline,
        Ok(RED::Esc) => ReedlineEvent::Esc,
        Ok(RED::Repaint) => ReedlineEvent::Repaint,
        Ok(RED::PreviousHistory) => ReedlineEvent::PreviousHistory,
        Ok(RED::Up) => ReedlineEvent::Up,
        Ok(RED::Down) => ReedlineEvent::Down,
        Ok(RED::Right) => ReedlineEvent::Right,
        Ok(RED::Left) => ReedlineEvent::Left,
        Ok(RED::ToStart) => ReedlineEvent::ToStart,
        Ok(RED::ToEnd) => ReedlineEvent::ToEnd,
        Ok(RED::NextHistory) => ReedlineEvent::NextHistory,
        Ok(RED::SearchHistory) => ReedlineEvent::SearchHistory,
        Ok(RED::Menu) => {
            let menu = extract_value("name", record, span)?;
            ReedlineEvent::Menu(menu.to_expanded_string("", config))
        }
        Ok(RED::MenuNext) => ReedlineEvent::MenuNext,
        Ok(RED::MenuPrevious) => ReedlineEvent::MenuPrevious,
        Ok(RED::MenuUp) => ReedlineEvent::MenuUp,
        Ok(RED::MenuDown) => ReedlineEvent::MenuDown,
        Ok(RED::MenuLeft) => ReedlineEvent::MenuLeft,
        Ok(RED::MenuRight) => ReedlineEvent::MenuRight,
        Ok(RED::MenuPageNext) => ReedlineEvent::MenuPageNext,
        Ok(RED::MenuPagePrevious) => ReedlineEvent::MenuPagePrevious,
        Ok(RED::ExecuteHostCommand) => {
            let cmd = extract_value("cmd", record, span)?;
            ReedlineEvent::ExecuteHostCommand(cmd.to_expanded_string("", config))
        }
        Ok(RED::OpenEditor) => ReedlineEvent::OpenEditor,
        Ok(RED::ViChangeMode) => {
            let mode = extract_value("mode", record, span)?;
            ReedlineEvent::ViChangeMode(mode.as_str()?.to_owned())
        }
        // Non-sensical for user configuration:
        //
        // `ReedlineEvent::Mouse` - itself a no-op
        // `ReedlineEvent::Resize` - requires size info specifically from the ANSI resize event
        //
        // Handled above in `parse_event`:
        //
        // `ReedlineEvent::Edit`
        // `ReedlineEvent::Multiple`
        // `ReedlineEvent::UntilFound`
        Ok(RED::Mouse | RED::Resize) | Ok(RED::Edit | RED::Multiple | RED::UntilFound) | Err(_) => {
            return Err(ShellError::InvalidValue {
                valid: "a reedline event".into(),
                actual: format!("'{name}'"),
                span,
            });
        }
    };

    Ok(event)
}

// This is displayed in `keybindings list` command
pub(crate) fn display_reedline_event(event: ReedlineEventDiscriminants) -> Option<&'static str> {
    use ReedlineEventDiscriminants as RED;
    Some(match event {
        RED::None => "None",
        RED::HistoryHintComplete => "HistoryHintComplete",
        RED::HistoryHintWordComplete => "HistoryHintWordComplete",
        RED::CtrlD => "CtrlD",
        RED::CtrlC => "CtrlC",
        RED::ClearScreen => "ClearScreen",
        RED::ClearScrollback => "ClearScrollback",
        RED::Enter => "Enter",
        RED::Submit => "Submit",
        RED::SubmitOrNewline => "SubmitOrNewline",
        RED::Esc => "Esc",
        RED::Edit => "event: { edit: <edit> }",
        RED::Repaint => "Repaint",
        RED::PreviousHistory => "PreviousHistory",
        RED::Up => "Up",
        RED::Down => "Down",
        RED::ToStart => "ToStart",
        RED::ToEnd => "ToEnd",
        RED::Right => "Right",
        RED::Left => "Left",
        RED::NextHistory => "NextHistory",
        RED::SearchHistory => "SearchHistory",
        RED::Multiple => "event: { send: list<event> }",
        RED::UntilFound => "event: { until: list<event> }",
        RED::Menu => "Menu name: <string>",
        RED::MenuNext => "MenuNext",
        RED::MenuPrevious => "MenuPrevious",
        RED::MenuUp => "MenuUp",
        RED::MenuDown => "MenuDown",
        RED::MenuLeft => "MenuLeft",
        RED::MenuRight => "MenuRight",
        RED::MenuPageNext => "MenuPageNext",
        RED::MenuPagePrevious => "MenuPagePrevious",
        RED::ExecuteHostCommand => "ExecuteHostCommand cmd: <string>",
        RED::OpenEditor => "OpenEditor",
        RED::ViChangeMode => "ViChangeMode mode: <string>",
        // Non-sensical for user configuration
        RED::Mouse | RED::Resize => return None,
    })
}

fn edit_from_record(
    name: &str,
    record: &Record,
    config: &Config,
    span: Span,
) -> Result<EditCommand, ShellError> {
    use EditCommandDiscriminants as ECD;
    // When updating this implementation, also update `display_edit_command` function
    let edit = match ECD::from_str(name) {
        Ok(ECD::MoveToStart) => EditCommand::MoveToStart {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveToLineStart) => EditCommand::MoveToLineStart {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveToLineNonBlankStart) => EditCommand::MoveToLineNonBlankStart {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveToEnd) => EditCommand::MoveToEnd {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveToLineEnd) => EditCommand::MoveToLineEnd {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveLineUp) => EditCommand::MoveLineUp {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveLineDown) => EditCommand::MoveLineDown {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveLeft) => EditCommand::MoveLeft {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveRight) => EditCommand::MoveRight {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveWordLeft) => EditCommand::MoveWordLeft {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveBigWordLeft) => EditCommand::MoveBigWordLeft {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveWordRight) => EditCommand::MoveWordRight {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveWordRightStart) => EditCommand::MoveWordRightStart {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveBigWordRightStart) => EditCommand::MoveBigWordRightStart {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveWordRightEnd) => EditCommand::MoveWordRightEnd {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveBigWordRightEnd) => EditCommand::MoveBigWordRightEnd {
            select: extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        },
        Ok(ECD::MoveToPosition) => {
            let value = extract_value("value", record, span)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            EditCommand::MoveToPosition {
                position: value.as_int()? as usize,
                select,
            }
        }
        Ok(ECD::InsertChar) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::InsertChar(char)
        }
        Ok(ECD::InsertString) => {
            let value = extract_value("value", record, span)?;
            EditCommand::InsertString(value.to_expanded_string("", config))
        }
        Ok(ECD::InsertNewline) => EditCommand::InsertNewline,
        Ok(ECD::InsertNewlineAbove) => EditCommand::InsertNewlineAbove,
        Ok(ECD::InsertNewlineBelow) => EditCommand::InsertNewlineBelow,
        Ok(ECD::ReplaceChar) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::ReplaceChar(char)
        }
        Ok(ECD::Backspace) => EditCommand::Backspace,
        Ok(ECD::Delete) => EditCommand::Delete,
        Ok(ECD::CutChar) => EditCommand::CutChar,
        Ok(ECD::BackspaceWord) => EditCommand::BackspaceWord,
        Ok(ECD::DeleteWord) => EditCommand::DeleteWord,
        Ok(ECD::Clear) => EditCommand::Clear,
        Ok(ECD::ClearToLineEnd) => EditCommand::ClearToLineEnd,
        Ok(ECD::Complete) => EditCommand::Complete,
        Ok(ECD::CutCurrentLine) => EditCommand::CutCurrentLine,
        Ok(ECD::CutFromStart) => EditCommand::CutFromStart,
        Ok(ECD::CutFromStartLinewise) => EditCommand::CutFromStartLinewise {
            leave_blank_line: extract_value("keep_line", record, span)
                .and_then(|value| value.as_bool())?,
        },
        Ok(ECD::CutFromLineStart) => EditCommand::CutFromLineStart,
        Ok(ECD::CutFromLineNonBlankStart) => EditCommand::CutFromLineNonBlankStart,
        Ok(ECD::CutToEnd) => EditCommand::CutToEnd,
        Ok(ECD::CutToEndLinewise) => EditCommand::CutToEndLinewise {
            leave_blank_line: extract_value("keep_line", record, span)
                .and_then(|value| value.as_bool())?,
        },
        Ok(ECD::CutToLineEnd) => EditCommand::CutToLineEnd,
        Ok(ECD::KillLine) => EditCommand::KillLine,
        Ok(ECD::CutWordLeft) => EditCommand::CutWordLeft,
        Ok(ECD::CutBigWordLeft) => EditCommand::CutBigWordLeft,
        Ok(ECD::CutWordRight) => EditCommand::CutWordRight,
        Ok(ECD::CutBigWordRight) => EditCommand::CutBigWordRight,
        Ok(ECD::CutWordRightToNext) => EditCommand::CutWordRightToNext,
        Ok(ECD::CutBigWordRightToNext) => EditCommand::CutBigWordRightToNext,
        Ok(ECD::PasteCutBufferBefore) => EditCommand::PasteCutBufferBefore,
        Ok(ECD::PasteCutBufferAfter) => EditCommand::PasteCutBufferAfter,
        Ok(ECD::UppercaseWord) => EditCommand::UppercaseWord,
        Ok(ECD::LowercaseWord) => EditCommand::LowercaseWord,
        Ok(ECD::CapitalizeChar) => EditCommand::CapitalizeChar,
        Ok(ECD::SwitchcaseChar) => EditCommand::SwitchcaseChar,
        Ok(ECD::SwapWords) => EditCommand::SwapWords,
        Ok(ECD::SwapGraphemes) => EditCommand::SwapGraphemes,
        Ok(ECD::Undo) => EditCommand::Undo,
        Ok(ECD::Redo) => EditCommand::Redo,
        Ok(ECD::CutRightUntil) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CutRightUntil(char)
        }
        Ok(ECD::CutRightBefore) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CutRightBefore(char)
        }
        Ok(ECD::MoveRightUntil) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveRightUntil { c: char, select }
        }
        Ok(ECD::MoveRightBefore) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveRightBefore { c: char, select }
        }
        Ok(ECD::CutLeftUntil) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CutLeftUntil(char)
        }
        Ok(ECD::CutLeftBefore) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CutLeftBefore(char)
        }
        Ok(ECD::MoveLeftUntil) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveLeftUntil { c: char, select }
        }
        Ok(ECD::MoveLeftBefore) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveLeftBefore { c: char, select }
        }
        Ok(ECD::SelectAll) => EditCommand::SelectAll,
        Ok(ECD::CutSelection) => EditCommand::CutSelection,
        Ok(ECD::CopySelection) => EditCommand::CopySelection,
        Ok(ECD::Paste) => EditCommand::Paste,
        Ok(ECD::CopyFromStart) => EditCommand::CopyFromStart,
        Ok(ECD::CopyFromStartLinewise) => EditCommand::CopyFromStartLinewise,
        Ok(ECD::CopyFromLineStart) => EditCommand::CopyFromLineStart,
        Ok(ECD::CopyFromLineNonBlankStart) => EditCommand::CopyFromLineNonBlankStart,
        Ok(ECD::CopyToEnd) => EditCommand::CopyToEnd,
        Ok(ECD::CopyToEndLinewise) => EditCommand::CopyToEndLinewise,
        Ok(ECD::CopyToLineEnd) => EditCommand::CopyToLineEnd,
        Ok(ECD::CopyCurrentLine) => EditCommand::CopyCurrentLine,
        Ok(ECD::CopyWordLeft) => EditCommand::CopyWordLeft,
        Ok(ECD::CopyBigWordLeft) => EditCommand::CopyBigWordLeft,
        Ok(ECD::CopyWordRight) => EditCommand::CopyWordRight,
        Ok(ECD::CopyBigWordRight) => EditCommand::CopyBigWordRight,
        Ok(ECD::CopyWordRightToNext) => EditCommand::CopyWordRightToNext,
        Ok(ECD::CopyBigWordRightToNext) => EditCommand::CopyBigWordRightToNext,
        Ok(ECD::CopyLeft) => EditCommand::CopyLeft,
        Ok(ECD::CopyRight) => EditCommand::CopyRight,
        Ok(ECD::CopyRightUntil) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CopyRightUntil(char)
        }
        Ok(ECD::CopyRightBefore) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CopyRightBefore(char)
        }
        Ok(ECD::CopyLeftUntil) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CopyLeftUntil(char)
        }
        Ok(ECD::CopyLeftBefore) => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value)?;
            EditCommand::CopyLeftBefore(char)
        }
        Ok(ECD::SwapCursorAndAnchor) => EditCommand::SwapCursorAndAnchor,
        #[cfg(feature = "system-clipboard")]
        Ok(ECD::CutSelectionSystem) => EditCommand::CutSelectionSystem,
        #[cfg(feature = "system-clipboard")]
        Ok(ECD::CopySelectionSystem) => EditCommand::CopySelectionSystem,
        #[cfg(feature = "system-clipboard")]
        Ok(ECD::PasteSystem) => EditCommand::PasteSystem,
        Ok(ECD::CutInsidePair) => {
            let value = extract_value("left", record, span)?;
            let left = extract_char(value)?;
            let value = extract_value("right", record, span)?;
            let right = extract_char(value)?;
            EditCommand::CutInsidePair { left, right }
        }
        Ok(ECD::CopyInsidePair) => {
            let value = extract_value("left", record, span)?;
            let left = extract_char(value)?;
            let value = extract_value("right", record, span)?;
            let right = extract_char(value)?;
            EditCommand::CopyInsidePair { left, right }
        }
        Ok(ECD::CutAroundPair) => {
            let value = extract_value("left", record, span)?;
            let left = extract_char(value)?;
            let value = extract_value("right", record, span)?;
            let right = extract_char(value)?;
            EditCommand::CutAroundPair { left, right }
        }
        Ok(ECD::CopyAroundPair) => {
            let value = extract_value("left", record, span)?;
            let left = extract_char(value)?;
            let value = extract_value("right", record, span)?;
            let right = extract_char(value)?;
            EditCommand::CopyAroundPair { left, right }
        }
        Ok(ECD::CopyTextObject) => EditCommand::CopyTextObject {
            text_object: parse_text_object(record, config, span)?,
        },
        Ok(ECD::CutTextObject) => EditCommand::CutTextObject {
            text_object: parse_text_object(record, config, span)?,
        },
        // The verb commands take a `MotionTarget` (and, for the operators, a
        // `Granularity`) parsed from the same record. See `parse_motion_target`.
        Ok(ECD::Move) => EditCommand::Move(parse_motion_target(record, config, span)?),
        Ok(ECD::Extend) => EditCommand::Extend(parse_motion_target(record, config, span)?),
        Ok(ECD::Erase) => EditCommand::Erase(parse_motion_target(record, config, span)?),
        Ok(ECD::Cut) => EditCommand::Cut {
            target: parse_motion_target(record, config, span)?,
            granularity: parse_granularity(record, config, span)?,
        },
        Ok(ECD::Copy) => EditCommand::Copy {
            target: parse_motion_target(record, config, span)?,
            granularity: parse_granularity(record, config, span)?,
        },
        Ok(ECD::Change) => EditCommand::Change {
            target: parse_motion_target(record, config, span)?,
            granularity: parse_granularity(record, config, span)?,
        },
        // `EditCommand::ReplaceChars` - Internal hack not sanely implementable as a
        // standalone binding
        Ok(ECD::ReplaceChars) | Err(_) => {
            return Err(ShellError::InvalidValue {
                valid: "a reedline EditCommand".into(),
                actual: format!("'{name}'"),
                span,
            });
        }
    };

    Ok(edit)
}

// This is displayed in `keybindings list` command
pub(crate) fn display_edit_command(edit: EditCommandDiscriminants) -> Option<&'static str> {
    use EditCommandDiscriminants as ECD;
    Some(match edit {
        ECD::MoveToStart => "MoveToStart select?: <bool>",
        ECD::MoveToLineStart => "MoveToLineStart select?: <bool>",
        ECD::MoveToLineNonBlankStart => "MoveToLineNonBlankStart select?: <bool>",
        ECD::MoveToEnd => "MoveToEnd select?: <bool>",
        ECD::MoveToLineEnd => "MoveToLineEnd select?: <bool>",
        ECD::MoveLineUp => "MoveLineUp select?: <bool>",
        ECD::MoveLineDown => "MoveLineDown select?: <bool>",
        ECD::MoveLeft => "MoveLeft select?: <bool>",
        ECD::MoveRight => "MoveRight select?: <bool>",
        ECD::MoveWordLeft => "MoveWordLeft select?: <bool>",
        ECD::MoveBigWordLeft => "MoveBigWordLeft select?: <bool>",
        ECD::MoveWordRight => "MoveWordRight select?: <bool>",
        ECD::MoveWordRightEnd => "MoveWordRightEnd select?: <bool>",
        ECD::MoveBigWordRightEnd => "MoveBigWordRightEnd select?: <bool>",
        ECD::MoveWordRightStart => "MoveWordRightStart select?: <bool>",
        ECD::MoveBigWordRightStart => "MoveBigWordRightStart select?: <bool>",
        ECD::MoveToPosition => "MoveToPosition value: <int>, select?: <bool>",
        ECD::MoveLeftUntil => "MoveLeftUntil value: <char>, select?: <bool>",
        ECD::MoveLeftBefore => "MoveLeftBefore value: <char>, select?: <bool>",
        ECD::InsertChar => "InsertChar value: <char>",
        ECD::InsertString => "InsertString value: <string>",
        ECD::InsertNewline => "InsertNewline",
        ECD::InsertNewlineAbove => "InsertNewlineAbove",
        ECD::InsertNewlineBelow => "InsertNewlineBelow",
        ECD::ReplaceChar => "ReplaceChar value: <char>",
        ECD::Backspace => "Backspace",
        ECD::Delete => "Delete",
        ECD::CutChar => "CutChar",
        ECD::BackspaceWord => "BackspaceWord",
        ECD::DeleteWord => "DeleteWord",
        ECD::Clear => "Clear",
        ECD::ClearToLineEnd => "ClearToLineEnd",
        ECD::Complete => "Complete",
        ECD::CutCurrentLine => "CutCurrentLine",
        ECD::CutFromStart => "CutFromStart",
        ECD::CutFromStartLinewise => "CutFromStartLinewise keep_line: <bool>",
        ECD::CutFromLineStart => "CutFromLineStart",
        ECD::CutFromLineNonBlankStart => "CutFromLineNonBlankStart",
        ECD::CutToEnd => "CutToEnd",
        ECD::CutToEndLinewise => "CutToEndLinewise keep_line: <bool>",
        ECD::CutToLineEnd => "CutToLineEnd",
        ECD::KillLine => "KillLine",
        ECD::CutWordLeft => "CutWordLeft",
        ECD::CutBigWordLeft => "CutBigWordLeft",
        ECD::CutWordRight => "CutWordRight",
        ECD::CutBigWordRight => "CutBigWordRight",
        ECD::CutWordRightToNext => "CutWordRightToNext",
        ECD::CutBigWordRightToNext => "CutBigWordRightToNext",
        ECD::PasteCutBufferBefore => "PasteCutBufferBefore",
        ECD::PasteCutBufferAfter => "PasteCutBufferAfter",
        ECD::UppercaseWord => "UppercaseWord",
        ECD::LowercaseWord => "LowercaseWord",
        ECD::SwitchcaseChar => "SwitchcaseChar",
        ECD::CapitalizeChar => "CapitalizeChar",
        ECD::SwapWords => "SwapWords",
        ECD::SwapGraphemes => "SwapGraphemes",
        ECD::Undo => "Undo",
        ECD::Redo => "Redo",
        ECD::CutRightUntil => "CutRightUntil value: <char>",
        ECD::CutRightBefore => "CutRightBefore value: <char>",
        ECD::MoveRightUntil => "MoveRightUntil value: <char>",
        ECD::MoveRightBefore => "MoveRightBefore value: <char>",
        ECD::CutLeftUntil => "CutLeftUntil value: <char>",
        ECD::CutLeftBefore => "CutLeftBefore value: <char>",
        ECD::SelectAll => "SelectAll",
        ECD::CutSelection => "CutSelection",
        ECD::CopySelection => "CopySelection",
        ECD::Paste => "Paste",
        ECD::CopyFromStart => "CopyFromStart",
        ECD::CopyFromStartLinewise => "CopyFromStartLinewise",
        ECD::CopyFromLineStart => "CopyFromLineStart",
        ECD::CopyFromLineNonBlankStart => "CopyFromLineNonBlankStart",
        ECD::CopyToEnd => "CopyToEnd",
        ECD::CopyToEndLinewise => "CopyToEndLinewise",
        ECD::CopyToLineEnd => "CopyToLineEnd",
        ECD::CopyCurrentLine => "CopyCurrentLine",
        ECD::CopyWordLeft => "CopyWordLeft",
        ECD::CopyBigWordLeft => "CopyBigWordLeft",
        ECD::CopyWordRight => "CopyWordRight",
        ECD::CopyBigWordRight => "CopyBigWordRight",
        ECD::CopyWordRightToNext => "CopyWordRightToNext",
        ECD::CopyBigWordRightToNext => "CopyBigWordRightToNext",
        ECD::CopyLeft => "CopyLeft",
        ECD::CopyRight => "CopyRight",
        ECD::CopyRightUntil => "CopyRightUntil value: <char>",
        ECD::CopyRightBefore => "CopyRightBefore value: <char>",
        ECD::CopyLeftUntil => "CopyLeftUntil value: <char>",
        ECD::CopyLeftBefore => "CopyLeftBefore value: <char>",
        ECD::SwapCursorAndAnchor => "SwapCursorAndAnchor",
        #[cfg(feature = "system-clipboard")]
        ECD::CutSelectionSystem => "CutSelectionSystem",
        #[cfg(feature = "system-clipboard")]
        ECD::CopySelectionSystem => "CopySelectionSystem",
        #[cfg(feature = "system-clipboard")]
        ECD::PasteSystem => "PasteSystem",
        ECD::CutInsidePair => "CutInsidePair left: <char>, right <char>",
        ECD::CopyInsidePair => "CopyInsidePair left: <char>, right <char>",
        ECD::CutAroundPair => "CutAroundPair left: <char>, right <char>",
        ECD::CopyAroundPair => "CopyAroundPair left: <char>, right <char>",
        ECD::CutTextObject => "CutTextObject scope: <string>, object_type: <string>",
        ECD::CopyTextObject => "CopyTextObject scope: <string>, object_type: <string>",
        ECD::Move => {
            "Move motion: <string>, direction: <string>, word_kind?: <string>, edge?: <string>, char?: <char>, stop?: <string>"
        }
        ECD::Extend => {
            "Extend motion: <string>, direction: <string>, word_kind?: <string>, edge?: <string>, char?: <char>, stop?: <string>"
        }
        ECD::Erase => {
            "Erase motion: <string>, direction: <string>, word_kind?: <string>, edge?: <string>, char?: <char>, stop?: <string>"
        }
        ECD::Cut => {
            "Cut motion: <string>, direction: <string>, word_kind?: <string>, edge?: <string>, char?: <char>, stop?: <string>, granularity?: <string>"
        }
        ECD::Copy => {
            "Copy motion: <string>, direction: <string>, word_kind?: <string>, edge?: <string>, char?: <char>, stop?: <string>, granularity?: <string>"
        }
        ECD::Change => {
            "Change motion: <string>, direction: <string>, word_kind?: <string>, edge?: <string>, char?: <char>, stop?: <string>, granularity?: <string>"
        }
        ECD::ReplaceChars => return None,
    })
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

fn parse_text_object(
    record: &Record,
    config: &Config,
    span: Span,
) -> Result<TextObject, ShellError> {
    let scope = extract_enum_field(
        "scope",
        record,
        config,
        span,
        "'inner' or 'around'",
        |name| match name {
            "inner" => Some(TextObjectScope::Inner),
            "around" => Some(TextObjectScope::Around),
            _ => None,
        },
    )?;

    let object_type = extract_enum_field(
        "object_type",
        record,
        config,
        span,
        "'word', 'bigword', 'brackets', or 'quote'",
        |name| match name {
            "word" => Some(TextObjectType::Word),
            "bigword" => Some(TextObjectType::BigWord),
            "brackets" | "bracket" => Some(TextObjectType::Brackets),
            "quote" | "quotes" => Some(TextObjectType::Quote),
            _ => None,
        },
    )?;

    Ok(TextObject { scope, object_type })
}

/// Read a lowercased string field from `record` and map it to an enum value,
/// erroring with `valid` if the field is missing or unrecognized.
fn extract_enum_field<T>(
    field: &'static str,
    record: &Record,
    config: &Config,
    span: Span,
    valid: &str,
    parse: impl Fn(&str) -> Option<T>,
) -> Result<T, ShellError> {
    let value = extract_value(field, record, span)?;
    let name = value.to_expanded_string("", config).to_ascii_lowercase();
    parse(&name).ok_or_else(|| ShellError::InvalidValue {
        valid: valid.into(),
        actual: format!("'{name}'"),
        span: value.span(),
    })
}

fn parse_direction(record: &Record, config: &Config, span: Span) -> Result<Direction, ShellError> {
    extract_enum_field(
        "direction",
        record,
        config,
        span,
        "'forward' or 'backward'",
        |name| match name {
            "forward" => Some(Direction::Forward),
            "backward" => Some(Direction::Backward),
            _ => None,
        },
    )
}

/// Operator span granularity; an absent field falls back to
/// `Granularity::default()` (char-wise).
fn parse_granularity(
    record: &Record,
    config: &Config,
    span: Span,
) -> Result<Granularity, ShellError> {
    if !record.contains("granularity") {
        return Ok(Granularity::default());
    }
    extract_enum_field(
        "granularity",
        record,
        config,
        span,
        "'charwise' or 'linewise'",
        |name| match name {
            "charwise" => Some(Granularity::CharWise),
            "linewise" => Some(Granularity::LineWise),
            _ => None,
        },
    )
}

/// Parse a [`MotionTarget`] from the flat fields of an edit-command record.
///
/// `motion` selects the variant; the remaining fields supply its data —
/// `direction` (forward/backward), `word_kind` (small/big), `edge` (start/end),
/// `char`, and `stop` (on/before).
///
/// `MotionTarget::Offset` is intentionally not exposed; `MoveToPosition`
/// already covers absolute cursor positions.
fn parse_motion_target(
    record: &Record,
    config: &Config,
    span: Span,
) -> Result<MotionTarget, ShellError> {
    let motion_value = extract_value("motion", record, span)?;
    let motion = motion_value
        .to_expanded_string("", config)
        .to_ascii_lowercase();
    let target = match motion.as_str() {
        "grapheme" => MotionTarget::Grapheme(parse_direction(record, config, span)?),
        "word" => MotionTarget::Word {
            kind: extract_enum_field(
                "word_kind",
                record,
                config,
                span,
                "'small' or 'big'",
                |name| match name {
                    "small" => Some(WordKind::Small),
                    "big" => Some(WordKind::Big),
                    _ => None,
                },
            )?,
            edge: extract_enum_field("edge", record, config, span, "'start' or 'end'", |name| {
                match name {
                    "start" => Some(WordEdge::Start),
                    "end" => Some(WordEdge::End),
                    _ => None,
                }
            })?,
            direction: parse_direction(record, config, span)?,
        },
        "lineedge" => MotionTarget::LineEdge(parse_direction(record, config, span)?),
        "bufferedge" => MotionTarget::BufferEdge(parse_direction(record, config, span)?),
        "line" => MotionTarget::Line(parse_direction(record, config, span)?),
        "find" => {
            let ch = extract_char(extract_value("char", record, span)?)?;
            MotionTarget::Find {
                ch,
                direction: parse_direction(record, config, span)?,
                stop: extract_enum_field(
                    "stop",
                    record,
                    config,
                    span,
                    "'on' or 'before'",
                    |name| match name {
                        "on" => Some(FindStop::On),
                        "before" => Some(FindStop::Before),
                        _ => None,
                    },
                )?,
            }
        }
        other => {
            return Err(ShellError::InvalidValue {
                valid: "a motion: 'grapheme', 'word', 'lineedge', 'bufferedge', 'line', or 'find'"
                    .into(),
                actual: format!("'{other}'"),
                span: motion_value.span(),
            });
        }
    };
    Ok(target)
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
    fn test_edit_move_motion() {
        let event = Value::test_record(record! {
            "edit" => Value::test_string("Move"),
            "motion" => Value::test_string("grapheme"),
            "direction" => Value::test_string("backward"),
        });
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            Some(ReedlineEvent::Edit(vec![EditCommand::Move(
                MotionTarget::Grapheme(Direction::Backward)
            )]))
        );
    }

    #[test]
    fn test_edit_cut_word_linewise() {
        let event = Value::test_record(record! {
            "edit" => Value::test_string("Cut"),
            "motion" => Value::test_string("word"),
            "word_kind" => Value::test_string("big"),
            "edge" => Value::test_string("end"),
            "direction" => Value::test_string("forward"),
            "granularity" => Value::test_string("linewise"),
        });
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            Some(ReedlineEvent::Edit(vec![EditCommand::Cut {
                target: MotionTarget::Word {
                    kind: WordKind::Big,
                    edge: WordEdge::End,
                    direction: Direction::Forward,
                },
                granularity: Granularity::LineWise,
            }]))
        );
    }

    #[test]
    fn test_edit_find_motion() {
        let event = Value::test_record(record! {
            "edit" => Value::test_string("Erase"),
            "motion" => Value::test_string("find"),
            "char" => Value::test_string("x"),
            "direction" => Value::test_string("forward"),
            "stop" => Value::test_string("before"),
        });
        let config = Config::default();

        let parsed_event = parse_event(&event, &config).unwrap();
        assert_eq!(
            parsed_event,
            Some(ReedlineEvent::Edit(vec![EditCommand::Erase(
                MotionTarget::Find {
                    ch: 'x',
                    direction: Direction::Forward,
                    stop: FindStop::Before,
                }
            )]))
        );
    }

    #[test]
    fn test_parse_input_mode() {
        let config = Config::default();
        assert!(matches!(
            parse_input_mode(&Value::test_string("full_buffer"), &config),
            Ok(InputMode::FullBuffer)
        ));
        assert!(matches!(
            parse_input_mode(&Value::test_string("diff"), &config),
            Ok(InputMode::Diff)
        ));
        assert!(parse_input_mode(&Value::test_string("nope"), &config).is_err());
    }

    fn test_menu(input_mode: Option<Value>, only_buffer_difference: Option<Value>) -> ParsedMenu {
        ParsedMenu {
            name: Value::test_string("test_menu"),
            marker: Value::test_string("| "),
            only_buffer_difference,
            input_mode,
            output_mode: None,
            style: Value::test_nothing(),
            r#type: Value::test_nothing(),
            source: None,
        }
    }

    #[test]
    fn test_resolve_input_mode() {
        let config = Config::default();

        // input_mode alone
        assert!(matches!(
            resolve_input_mode(
                &test_menu(Some(Value::test_string("full_buffer")), None),
                &config
            ),
            Ok(InputMode::FullBuffer)
        ));

        // input_mode supersedes a conflicting legacy flag
        assert!(matches!(
            resolve_input_mode(
                &test_menu(
                    Some(Value::test_string("diff")),
                    Some(Value::test_bool(false))
                ),
                &config
            ),
            Ok(InputMode::Diff)
        ));

        // legacy flag alone maps to the equivalent mode
        assert!(matches!(
            resolve_input_mode(&test_menu(None, Some(Value::test_bool(true))), &config),
            Ok(InputMode::Diff)
        ));
        assert!(matches!(
            resolve_input_mode(&test_menu(None, Some(Value::test_bool(false))), &config),
            Ok(InputMode::CursorPrefix)
        ));

        // neither field set is a config error
        assert!(matches!(
            resolve_input_mode(&test_menu(None, None), &config),
            Err(ShellError::MissingRequiredColumn { .. })
        ));
    }

    #[test]
    fn test_parse_output_mode() {
        let config = Config::default();
        assert!(matches!(
            parse_output_mode(&Value::test_string("extend_to_end"), &config),
            Ok(OutputMode::ExtendToEnd)
        ));
        assert!(matches!(
            parse_output_mode(&Value::test_string("suggested_span"), &config),
            Ok(OutputMode::SuggestedSpan)
        ));
        assert!(parse_output_mode(&Value::test_string("nope"), &config).is_err());
    }

    #[test]
    fn test_parse_description_position() {
        let config = Config::default();
        assert!(matches!(
            parse_description_position(&Value::test_string("after"), &config),
            Ok(DescriptionPosition::After)
        ));
        assert!(matches!(
            parse_description_position(&Value::test_string("before"), &config),
            Ok(DescriptionPosition::Before)
        ));
        assert!(parse_description_position(&Value::test_string("nope"), &config).is_err());
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
