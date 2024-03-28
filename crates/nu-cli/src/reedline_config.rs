use crate::{menus::NuMenuCompleter, NuHelpCompleter};
use crossterm::event::{KeyCode, KeyModifiers};
use log::trace;
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    create_menus,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    extract_value, Config, EditBindings, ParsedKeybinding, ParsedMenu, PipelineData, Record,
    ShellError, Span, Value,
};
use reedline::{
    default_emacs_keybindings, default_vi_insert_keybindings, default_vi_normal_keybindings,
    ColumnarMenu, DescriptionMenu, DescriptionMode, EditCommand, IdeMenu, Keybindings, ListMenu,
    MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu,
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
    trace!("add_menus: config: {:#?}", &config);
    line_editor = line_editor.clear_menus();

    for menu in &config.menus {
        line_editor = add_menu(line_editor, menu, engine_state.clone(), stack, config)?
    }

    // Checking if the default menus have been added from the config file
    let default_menus = [
        ("completion_menu", DEFAULT_COMPLETION_MENU),
        ("history_menu", DEFAULT_HISTORY_MENU),
        ("help_menu", DEFAULT_HELP_MENU),
    ];

    for (name, definition) in default_menus {
        if !config
            .menus
            .iter()
            .any(|menu| menu.name.to_expanded_string("", config) == name)
        {
            let (block, _) = {
                let mut working_set = StateWorkingSet::new(&engine_state);
                let output = parse(
                    &mut working_set,
                    Some(name), // format!("entry #{}", entry_num)
                    definition.as_bytes(),
                    true,
                );

                (output, working_set.render())
            };

            let mut temp_stack = Stack::new().capture();
            let input = PipelineData::Empty;
            let res = eval_block::<WithoutDebug>(&engine_state, &mut temp_stack, &block, input)?;

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
    let span = menu.menu_type.span();
    if let Value::Record { val, .. } = &menu.menu_type {
        let layout = extract_value("layout", val, span)?.to_expanded_string("", config);

        match layout.as_str() {
            "columnar" => add_columnar_menu(line_editor, menu, engine_state, stack, config),
            "list" => add_list_menu(line_editor, menu, engine_state, stack, config),
            "ide" => add_ide_menu(line_editor, menu, engine_state, stack, config),
            "description" => add_description_menu(line_editor, menu, engine_state, stack, config),
            _ => Err(ShellError::UnsupportedConfigValue {
                expected: "columnar, list, ide or description".to_string(),
                value: menu.menu_type.to_abbreviated_string(config),
                span: menu.menu_type.span(),
            }),
        }
    } else {
        Err(ShellError::UnsupportedConfigValue {
            expected: "only record type".to_string(),
            value: menu.menu_type.to_abbreviated_string(config),
            span: menu.menu_type.span(),
        })
    }
}

macro_rules! add_style {
    // first arm match add!(1,2), add!(2,3) etc
    ($name:expr, $record: expr, $span:expr, $config: expr, $menu:expr, $f:expr) => {
        $menu = match extract_value($name, $record, $span) {
            Ok(text) => {
                let style = match text {
                    Value::String { val, .. } => lookup_ansi_color_style(&val),
                    Value::Record { .. } => color_record_to_nustyle(&text),
                    _ => lookup_ansi_color_style("green"),
                };
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
    let span = menu.menu_type.span();
    let name = menu.name.to_expanded_string("", config);
    let mut columnar_menu = ColumnarMenu::default().with_name(&name);

    if let Value::Record { val, .. } = &menu.menu_type {
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

    let span = menu.style.span();
    if let Value::Record { val, .. } = &menu.style {
        add_style!(
            "text",
            val,
            span,
            config,
            columnar_menu,
            ColumnarMenu::with_text_style
        );
        add_style!(
            "selected_text",
            val,
            span,
            config,
            columnar_menu,
            ColumnarMenu::with_selected_text_style
        );
        add_style!(
            "description_text",
            val,
            span,
            config,
            columnar_menu,
            ColumnarMenu::with_description_text_style
        );
        add_style!(
            "match_text",
            val,
            span,
            config,
            columnar_menu,
            ColumnarMenu::with_match_text_style
        );
        add_style!(
            "selected_match_text",
            val,
            span,
            config,
            columnar_menu,
            ColumnarMenu::with_selected_match_text_style
        );
    }

    let marker = menu.marker.to_expanded_string("", config);
    columnar_menu = columnar_menu.with_marker(&marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    columnar_menu = columnar_menu.with_only_buffer_difference(only_buffer_difference);

    let span = menu.source.span();
    match &menu.source {
        Value::Nothing { .. } => {
            Ok(line_editor.with_menu(ReedlineMenu::EngineCompleter(Box::new(columnar_menu))))
        }
        Value::Closure { val, .. } => {
            let menu_completer = NuMenuCompleter::new(
                val.block_id,
                span,
                stack.captures_to_stack(val.captures.clone()),
                engine_state,
                only_buffer_difference,
            );
            Ok(line_editor.with_menu(ReedlineMenu::WithCompleter {
                menu: Box::new(columnar_menu),
                completer: Box::new(menu_completer),
            }))
        }
        _ => Err(ShellError::UnsupportedConfigValue {
            expected: "block or omitted value".to_string(),
            value: menu.source.to_abbreviated_string(config),
            span,
        }),
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
    let name = menu.name.to_expanded_string("", config);
    let mut list_menu = ListMenu::default().with_name(&name);

    let span = menu.menu_type.span();
    if let Value::Record { val, .. } = &menu.menu_type {
        list_menu = match extract_value("page_size", val, span) {
            Ok(page_size) => {
                let page_size = page_size.as_int()?;
                list_menu.with_page_size(page_size as usize)
            }
            Err(_) => list_menu,
        };
    }

    let span = menu.style.span();
    if let Value::Record { val, .. } = &menu.style {
        add_style!(
            "text",
            val,
            span,
            config,
            list_menu,
            ListMenu::with_text_style
        );
        add_style!(
            "selected_text",
            val,
            span,
            config,
            list_menu,
            ListMenu::with_selected_text_style
        );
        add_style!(
            "description_text",
            val,
            span,
            config,
            list_menu,
            ListMenu::with_description_text_style
        );
    }

    let marker = menu.marker.to_expanded_string("", config);
    list_menu = list_menu.with_marker(&marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    list_menu = list_menu.with_only_buffer_difference(only_buffer_difference);

    let span = menu.source.span();
    match &menu.source {
        Value::Nothing { .. } => {
            Ok(line_editor.with_menu(ReedlineMenu::HistoryMenu(Box::new(list_menu))))
        }
        Value::Closure { val, .. } => {
            let menu_completer = NuMenuCompleter::new(
                val.block_id,
                span,
                stack.captures_to_stack(val.captures.clone()),
                engine_state,
                only_buffer_difference,
            );
            Ok(line_editor.with_menu(ReedlineMenu::WithCompleter {
                menu: Box::new(list_menu),
                completer: Box::new(menu_completer),
            }))
        }
        _ => Err(ShellError::UnsupportedConfigValue {
            expected: "block or omitted value".to_string(),
            value: menu.source.to_abbreviated_string(config),
            span: menu.source.span(),
        }),
    }
}

// Adds an IDE menu to the line editor
pub(crate) fn add_ide_menu(
    line_editor: Reedline,
    menu: &ParsedMenu,
    engine_state: Arc<EngineState>,
    stack: &Stack,
    config: &Config,
) -> Result<Reedline, ShellError> {
    let span = menu.menu_type.span();
    let name = menu.name.to_expanded_string("", config);
    let mut ide_menu = IdeMenu::default().with_name(&name);

    if let Value::Record { val, .. } = &menu.menu_type {
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
                    return Err(ShellError::UnsupportedConfigValue {
                        expected: "bool or record".to_string(),
                        value: border.to_abbreviated_string(config),
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
                _ => {
                    return Err(ShellError::UnsupportedConfigValue {
                        expected: "\"left\", \"right\" or \"prefer_right\"".to_string(),
                        value: description_mode.to_abbreviated_string(config),
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

    let span = menu.style.span();
    if let Value::Record { val, .. } = &menu.style {
        add_style!(
            "text",
            val,
            span,
            config,
            ide_menu,
            IdeMenu::with_text_style
        );
        add_style!(
            "selected_text",
            val,
            span,
            config,
            ide_menu,
            IdeMenu::with_selected_text_style
        );
        add_style!(
            "description_text",
            val,
            span,
            config,
            ide_menu,
            IdeMenu::with_description_text_style
        );
        add_style!(
            "match_text",
            val,
            span,
            config,
            ide_menu,
            IdeMenu::with_match_text_style
        );
        add_style!(
            "selected_match_text",
            val,
            span,
            config,
            ide_menu,
            IdeMenu::with_selected_match_text_style
        );
    }

    let marker = menu.marker.to_expanded_string("", config);
    ide_menu = ide_menu.with_marker(&marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    ide_menu = ide_menu.with_only_buffer_difference(only_buffer_difference);

    let span = menu.source.span();
    match &menu.source {
        Value::Nothing { .. } => {
            Ok(line_editor.with_menu(ReedlineMenu::EngineCompleter(Box::new(ide_menu))))
        }
        Value::Closure { val, .. } => {
            let menu_completer = NuMenuCompleter::new(
                val.block_id,
                span,
                stack.captures_to_stack(val.captures.clone()),
                engine_state,
                only_buffer_difference,
            );
            Ok(line_editor.with_menu(ReedlineMenu::WithCompleter {
                menu: Box::new(ide_menu),
                completer: Box::new(menu_completer),
            }))
        }
        _ => Err(ShellError::UnsupportedConfigValue {
            expected: "block or omitted value".to_string(),
            value: menu.source.to_abbreviated_string(config),
            span,
        }),
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
    let name = menu.name.to_expanded_string("", config);
    let mut description_menu = DescriptionMenu::default().with_name(&name);

    let span = menu.menu_type.span();
    if let Value::Record { val, .. } = &menu.menu_type {
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

    let span = menu.style.span();
    if let Value::Record { val, .. } = &menu.style {
        add_style!(
            "text",
            val,
            span,
            config,
            description_menu,
            DescriptionMenu::with_text_style
        );
        add_style!(
            "selected_text",
            val,
            span,
            config,
            description_menu,
            DescriptionMenu::with_selected_text_style
        );
        add_style!(
            "description_text",
            val,
            span,
            config,
            description_menu,
            DescriptionMenu::with_description_text_style
        );
    }

    let marker = menu.marker.to_expanded_string("", config);
    description_menu = description_menu.with_marker(&marker);

    let only_buffer_difference = menu.only_buffer_difference.as_bool()?;
    description_menu = description_menu.with_only_buffer_difference(only_buffer_difference);

    let span = menu.source.span();
    match &menu.source {
        Value::Nothing { .. } => {
            let completer = Box::new(NuHelpCompleter::new(engine_state));
            Ok(line_editor.with_menu(ReedlineMenu::WithCompleter {
                menu: Box::new(description_menu),
                completer,
            }))
        }
        Value::Closure { val, .. } => {
            let menu_completer = NuMenuCompleter::new(
                val.block_id,
                span,
                stack.captures_to_stack(val.captures.clone()),
                engine_state,
                only_buffer_difference,
            );
            Ok(line_editor.with_menu(ReedlineMenu::WithCompleter {
                menu: Box::new(description_menu),
                completer: Box::new(menu_completer),
            }))
        }
        _ => Err(ShellError::UnsupportedConfigValue {
            expected: "closure or omitted value".to_string(),
            value: menu.source.to_abbreviated_string(config),
            span: menu.source.span(),
        }),
    }
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
            "emacs" => add_parsed_keybinding(emacs_keybindings, keybinding, config),
            "vi_insert" => add_parsed_keybinding(insert_keybindings, keybinding, config),
            "vi_normal" => add_parsed_keybinding(normal_keybindings, keybinding, config),
            m => Err(ShellError::UnsupportedConfigValue {
                expected: "emacs, vi_insert or vi_normal".to_string(),
                value: m.to_string(),
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
        v => Err(ShellError::UnsupportedConfigValue {
            expected: "string or list of strings".to_string(),
            value: v.to_abbreviated_string(config),
            span: v.span(),
        }),
    }
}

fn add_parsed_keybinding(
    keybindings: &mut Keybindings,
    keybinding: &ParsedKeybinding,
    config: &Config,
) -> Result<(), ShellError> {
    let modifier = match keybinding
        .modifier
        .to_expanded_string("", config)
        .to_ascii_lowercase()
        .as_str()
    {
        "control" => KeyModifiers::CONTROL,
        "shift" => KeyModifiers::SHIFT,
        "alt" => KeyModifiers::ALT,
        "none" => KeyModifiers::NONE,
        "shift_alt" | "alt_shift" => KeyModifiers::SHIFT | KeyModifiers::ALT,
        "control_shift" | "shift_control" => KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        "control_alt" | "alt_control" => KeyModifiers::CONTROL | KeyModifiers::ALT,
        "control_alt_shift" | "control_shift_alt" => {
            KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
        }
        _ => {
            return Err(ShellError::UnsupportedConfigValue {
                expected: "CONTROL, SHIFT, ALT or NONE".to_string(),
                value: keybinding.modifier.to_abbreviated_string(config),
                span: keybinding.modifier.span(),
            })
        }
    };

    let keycode = match keybinding
        .keycode
        .to_expanded_string("", config)
        .to_ascii_lowercase()
        .as_str()
    {
        "backspace" => KeyCode::Backspace,
        "enter" => KeyCode::Enter,
        c if c.starts_with("char_") => {
            let mut char_iter = c.chars().skip(5);
            let pos1 = char_iter.next();
            let pos2 = char_iter.next();

            let char = if let (Some(char), None) = (pos1, pos2) {
                char
            } else {
                return Err(ShellError::UnsupportedConfigValue {
                    expected: "char_<CHAR: unicode codepoint>".to_string(),
                    value: c.to_string(),
                    span: keybinding.keycode.span(),
                });
            };

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
                .filter(|num| matches!(num, 1..=20))
                .ok_or(ShellError::UnsupportedConfigValue {
                    expected: "(f1|f2|...|f20)".to_string(),
                    value: format!("unknown function key: {c}"),
                    span: keybinding.keycode.span(),
                })?;
            KeyCode::F(fn_num)
        }
        "null" => KeyCode::Null,
        "esc" | "escape" => KeyCode::Esc,
        _ => {
            return Err(ShellError::UnsupportedConfigValue {
                expected: "crossterm KeyCode".to_string(),
                value: keybinding.keycode.to_abbreviated_string(config),
                span: keybinding.keycode.span(),
            })
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
            .map_err(|_| ShellError::MissingConfigValue {
                missing_value: "send, edit or until".to_string(),
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
                                None => Err(ShellError::UnsupportedConfigValue {
                                    expected: "List containing valid events".to_string(),
                                    value: "Nothing value (null)".to_string(),
                                    span: value.span(),
                                }),
                                Some(event) => Ok(event),
                            },
                            Err(e) => Err(e),
                        })
                        .collect::<Result<Vec<ReedlineEvent>, ShellError>>()?;

                    Ok(Some(ReedlineEvent::UntilFound(events)))
                }
                v => Err(ShellError::UnsupportedConfigValue {
                    expected: "list of events".to_string(),
                    value: v.to_abbreviated_string(config),
                    span: v.span(),
                }),
            },
        },
        Value::List { vals, .. } => {
            let events = vals
                .iter()
                .map(|value| match parse_event(value, config) {
                    Ok(inner) => match inner {
                        None => Err(ShellError::UnsupportedConfigValue {
                            expected: "List containing valid events".to_string(),
                            value: "Nothing value (null)".to_string(),
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
        v => Err(ShellError::UnsupportedConfigValue {
            expected: "record or list of records, null to unbind key".to_string(),
            value: v.to_abbreviated_string(config),
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
            let menu = extract_value("name", record, span)?;
            ReedlineEvent::Menu(menu.to_expanded_string("", config))
        }
        "executehostcommand" => {
            let cmd = extract_value("cmd", record, span)?;
            ReedlineEvent::ExecuteHostCommand(cmd.to_expanded_string("", config))
        }
        v => {
            return Err(ShellError::UnsupportedConfigValue {
                expected: "Reedline event".to_string(),
                value: v.to_string(),
                span,
            })
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
            let char = extract_char(value, config)?;
            EditCommand::InsertChar(char)
        }
        "insertstring" => {
            let value = extract_value("value", record, span)?;
            EditCommand::InsertString(value.to_expanded_string("", config))
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
            let value = extract_value("value", record, span)?;
            let char = extract_char(value, config)?;
            EditCommand::CutRightUntil(char)
        }
        "cutrightbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value, config)?;
            EditCommand::CutRightBefore(char)
        }
        "moverightuntil" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value, config)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveRightUntil { c: char, select }
        }
        "moverightbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value, config)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveRightBefore { c: char, select }
        }
        "cutleftuntil" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value, config)?;
            EditCommand::CutLeftUntil(char)
        }
        "cutleftbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value, config)?;
            EditCommand::CutLeftBefore(char)
        }
        "moveleftuntil" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value, config)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveLeftUntil { c: char, select }
        }
        "moveleftbefore" => {
            let value = extract_value("value", record, span)?;
            let char = extract_char(value, config)?;
            let select = extract_value("select", record, span)
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            EditCommand::MoveLeftBefore { c: char, select }
        }
        "complete" => EditCommand::Complete,
        "cutselection" => EditCommand::CutSelection,
        #[cfg(feature = "system-clipboard")]
        "cutselectionsystem" => EditCommand::CutSelectionSystem,
        "copyselection" => EditCommand::CopySelection,
        #[cfg(feature = "system-clipboard")]
        "copyselectionsystem" => EditCommand::CopySelectionSystem,
        "paste" => EditCommand::Paste,
        #[cfg(feature = "system-clipboard")]
        "pastesystem" => EditCommand::PasteSystem,
        "selectall" => EditCommand::SelectAll,
        e => {
            return Err(ShellError::UnsupportedConfigValue {
                expected: "reedline EditCommand".to_string(),
                value: e.to_string(),
                span,
            })
        }
    };

    Ok(edit)
}

fn extract_char(value: &Value, config: &Config) -> Result<char, ShellError> {
    let span = value.span();
    value
        .to_expanded_string("", config)
        .chars()
        .next()
        .ok_or_else(|| ShellError::MissingConfigValue {
            missing_value: "char to insert".to_string(),
            span,
        })
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
        assert!(matches!(b, Err(ShellError::MissingConfigValue { .. })));
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
