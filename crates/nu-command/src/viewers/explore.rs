use nu_ansi_term::{Color, Style};
use nu_color_config::{get_color_map, StyleComputer};
use nu_engine::CallExt;
use nu_explore::{
    run_pager,
    util::{create_map, map_into_value},
    PagerConfig, StyleConfig,
};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use std::collections::HashMap;

/// A `less` like program to render a [Value] as a table.
#[derive(Clone)]
pub struct Explore;

impl Command for Explore {
    fn name(&self) -> &str {
        "explore"
    }

    fn usage(&self) -> &str {
        "Explore acts as a table pager, just like `less` does for text"
    }

    fn signature(&self) -> nu_protocol::Signature {
        // todo: Fix error message when it's empty
        // if we set h i short flags it panics????

        Signature::build("explore")
            .named(
                "head",
                SyntaxShape::Boolean,
                "Show or hide column headers (default true)",
                None,
            )
            .switch("index", "Show row indexes when viewing a list", Some('i'))
            .switch(
                "reverse",
                "Start with the viewport scrolled to the bottom",
                Some('r'),
            )
            .switch(
                "peek",
                "When quitting, output the value of the cell the cursor was on",
                Some('p'),
            )
            .category(Category::Viewers)
    }

    fn extra_usage(&self) -> &str {
        r#"Press <:> then <h> to get a help menu."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let show_head: bool = call.get_flag(engine_state, stack, "head")?.unwrap_or(true);
        let show_index: bool = call.has_flag("index");
        let is_reverse: bool = call.has_flag("reverse");
        let peek_value: bool = call.has_flag("peek");

        let ctrlc = engine_state.ctrlc.clone();
        let nu_config = engine_state.get_config();
        let style_computer = StyleComputer::from_config(engine_state, stack);

        let mut config = nu_config.explore.clone();
        prepare_default_config(&mut config);
        update_config(&mut config, show_index, show_head);
        include_nu_config(&mut config, &style_computer);

        let show_banner = is_need_banner(&config).unwrap_or(true);
        let exit_esc = is_need_esc_exit(&config).unwrap_or(true);

        let style = style_from_config(&config);

        let lscolors = nu_explore::util::create_lscolors(engine_state, stack);

        let mut config = PagerConfig::new(nu_config, &style_computer, &lscolors, config);
        config.style = style;
        config.reverse = is_reverse;
        config.peek_value = peek_value;
        config.reverse = is_reverse;
        config.exit_esc = exit_esc;
        config.show_banner = show_banner;

        let result = run_pager(engine_state, &mut stack.clone(), ctrlc, input, config);

        match result {
            Ok(Some(value)) => Ok(PipelineData::Value(value, None)),
            Ok(None) => Ok(PipelineData::Value(Value::default(), None)),
            Err(err) => Ok(PipelineData::Value(
                Value::Error { error: err.into() },
                None,
            )),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Explore the system information record",
                example: r#"sys | explore"#,
                result: None,
            },
            Example {
                description: "Explore the output of `ls` without column names",
                example: r#"ls | explore --head false"#,
                result: None,
            },
            Example {
                description: "Explore a list of Markdown files' contents, with row indexes",
                example: r#"glob *.md | each { open } | explore -i"#,
                result: None,
            },
            Example {
                description:
                    "Explore a JSON file, then save the last visited sub-structure to a file",
                example: r#"open file.json | explore -p | to json | save part.json"#,
                result: None,
            },
        ]
    }
}

// For now, this doesn't use StyleComputer.
// As such, closures can't be given as styles for Explore.
fn is_need_banner(config: &HashMap<String, Value>) -> Option<bool> {
    config.get("help_banner").and_then(|v| v.as_bool().ok())
}

fn is_need_esc_exit(config: &HashMap<String, Value>) -> Option<bool> {
    config.get("exit_esc").and_then(|v| v.as_bool().ok())
}

fn update_config(config: &mut HashMap<String, Value>, show_index: bool, show_head: bool) {
    let mut hm = config.get("table").and_then(create_map).unwrap_or_default();
    if show_index {
        insert_bool(&mut hm, "show_index", show_index);
    }

    if show_head {
        insert_bool(&mut hm, "show_head", show_head);
    }

    config.insert(String::from("table"), map_into_value(hm));
}

fn style_from_config(config: &HashMap<String, Value>) -> StyleConfig {
    let mut style = StyleConfig::default();

    let colors = get_color_map(config);

    if let Some(s) = colors.get("status_bar_text") {
        style.status_bar_text = *s;
    }

    if let Some(s) = colors.get("status_bar_background") {
        style.status_bar_background = *s;
    }

    if let Some(s) = colors.get("command_bar_text") {
        style.cmd_bar_text = *s;
    }

    if let Some(s) = colors.get("command_bar_background") {
        style.cmd_bar_background = *s;
    }

    if let Some(hm) = config.get("status").and_then(create_map) {
        let colors = get_color_map(&hm);

        if let Some(s) = colors.get("info") {
            style.status_info = *s;
        }

        if let Some(s) = colors.get("warn") {
            style.status_warn = *s;
        }

        if let Some(s) = colors.get("error") {
            style.status_error = *s;
        }
    }

    style
}

fn prepare_default_config(config: &mut HashMap<String, Value>) {
    const STATUS_BAR: Style = color(
        Some(Color::Rgb(29, 31, 33)),
        Some(Color::Rgb(196, 201, 198)),
    );

    const INPUT_BAR: Style = color(Some(Color::Rgb(196, 201, 198)), None);

    const HIGHLIGHT: Style = color(Some(Color::Black), Some(Color::Yellow));

    const STATUS_ERROR: Style = color(Some(Color::White), Some(Color::Red));

    const STATUS_INFO: Style = color(None, None);

    const STATUS_WARN: Style = color(None, None);

    const TABLE_SPLIT_LINE: Style = color(Some(Color::Rgb(64, 64, 64)), None);

    const TABLE_LINE_HEADER_TOP: bool = true;

    const TABLE_LINE_HEADER_BOTTOM: bool = true;

    const TABLE_LINE_INDEX: bool = true;

    const TABLE_LINE_SHIFT: bool = true;

    const TABLE_SELECT_CURSOR: bool = true;

    const TABLE_SELECT_CELL: Style = color(None, None);

    const TABLE_SELECT_ROW: Style = color(None, None);

    const TABLE_SELECT_COLUMN: Style = color(None, None);

    const TRY_BORDER_COLOR: Style = color(None, None);

    const CONFIG_CURSOR_COLOR: Style = color(Some(Color::Black), Some(Color::LightYellow));

    insert_style(config, "status_bar_background", STATUS_BAR);
    insert_style(config, "command_bar_text", INPUT_BAR);
    insert_style(config, "highlight", HIGHLIGHT);

    // because how config works we need to parse a string into Value::Record

    {
        let mut hm = config
            .get("status")
            .and_then(parse_hash_map)
            .unwrap_or_default();

        insert_style(&mut hm, "info", STATUS_INFO);
        insert_style(&mut hm, "warn", STATUS_WARN);
        insert_style(&mut hm, "error", STATUS_ERROR);

        config.insert(String::from("status"), map_into_value(hm));
    }

    {
        let mut hm = config
            .get("table")
            .and_then(parse_hash_map)
            .unwrap_or_default();

        insert_style(&mut hm, "split_line", TABLE_SPLIT_LINE);
        insert_style(&mut hm, "selected_cell", TABLE_SELECT_CELL);
        insert_style(&mut hm, "selected_row", TABLE_SELECT_ROW);
        insert_style(&mut hm, "selected_column", TABLE_SELECT_COLUMN);
        insert_bool(&mut hm, "cursor", TABLE_SELECT_CURSOR);
        insert_bool(&mut hm, "line_head_top", TABLE_LINE_HEADER_TOP);
        insert_bool(&mut hm, "line_head_bottom", TABLE_LINE_HEADER_BOTTOM);
        insert_bool(&mut hm, "line_shift", TABLE_LINE_SHIFT);
        insert_bool(&mut hm, "line_index", TABLE_LINE_INDEX);

        config.insert(String::from("table"), map_into_value(hm));
    }

    {
        let mut hm = config
            .get("try")
            .and_then(parse_hash_map)
            .unwrap_or_default();

        insert_style(&mut hm, "border_color", TRY_BORDER_COLOR);

        config.insert(String::from("try"), map_into_value(hm));
    }

    {
        let mut hm = config
            .get("config")
            .and_then(parse_hash_map)
            .unwrap_or_default();

        insert_style(&mut hm, "cursor_color", CONFIG_CURSOR_COLOR);

        config.insert(String::from("config"), map_into_value(hm));
    }
}

fn parse_hash_map(value: &Value) -> Option<HashMap<String, Value>> {
    value
        .as_string()
        .ok()
        .and_then(|s| nu_json::from_str::<nu_json::Value>(&s).ok())
        .map(convert_json_value_into_value)
        .and_then(|v| create_map(&v))
}

const fn color(foreground: Option<Color>, background: Option<Color>) -> Style {
    Style {
        background,
        foreground,
        is_blink: false,
        is_bold: false,
        is_dimmed: false,
        is_hidden: false,
        is_italic: false,
        is_reverse: false,
        is_strikethrough: false,
        is_underline: false,
    }
}

fn insert_style(map: &mut HashMap<String, Value>, key: &str, value: Style) {
    if map.contains_key(key) {
        return;
    }

    if value == Style::default() {
        return;
    }

    let value = nu_color_config::NuStyle::from(value);
    if let Ok(val) = nu_json::to_string_raw(&value) {
        map.insert(String::from(key), Value::string(val, Span::unknown()));
    }
}

fn insert_bool(map: &mut HashMap<String, Value>, key: &str, value: bool) {
    if map.contains_key(key) {
        return;
    }

    map.insert(String::from(key), Value::boolean(value, Span::unknown()));
}

fn convert_json_value_into_value(value: nu_json::Value) -> Value {
    match value {
        nu_json::Value::Null => Value::nothing(Span::unknown()),
        nu_json::Value::Bool(val) => Value::boolean(val, Span::unknown()),
        nu_json::Value::I64(val) => Value::int(val, Span::unknown()),
        nu_json::Value::U64(val) => Value::int(val as i64, Span::unknown()),
        nu_json::Value::F64(val) => Value::float(val, Span::unknown()),
        nu_json::Value::String(val) => Value::string(val, Span::unknown()),
        nu_json::Value::Array(val) => {
            let vals = val
                .into_iter()
                .map(convert_json_value_into_value)
                .collect::<Vec<_>>();

            Value::List {
                vals,
                span: Span::unknown(),
            }
        }
        nu_json::Value::Object(val) => {
            let hm = val
                .into_iter()
                .map(|(key, value)| {
                    let val = convert_json_value_into_value(value);
                    (key, val)
                })
                .collect();

            map_into_value(hm)
        }
    }
}

fn include_nu_config(config: &mut HashMap<String, Value>, style_computer: &StyleComputer) {
    let line_color = lookup_color(style_computer, "separator");
    if line_color != nu_ansi_term::Style::default() {
        {
            let mut map = config
                .get("table")
                .and_then(parse_hash_map)
                .unwrap_or_default();
            insert_style(&mut map, "split_line", line_color);
            config.insert(String::from("table"), map_into_value(map));
        }

        {
            let mut map = config
                .get("try")
                .and_then(parse_hash_map)
                .unwrap_or_default();
            insert_style(&mut map, "border_color", line_color);
            config.insert(String::from("try"), map_into_value(map));
        }

        {
            let mut map = config
                .get("config")
                .and_then(parse_hash_map)
                .unwrap_or_default();
            insert_style(&mut map, "border_color", line_color);
            config.insert(String::from("config"), map_into_value(map));
        }
    }
}

fn lookup_color(style_computer: &StyleComputer, key: &str) -> nu_ansi_term::Style {
    style_computer.compute(key, &Value::nothing(Span::unknown()))
}
