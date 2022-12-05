use std::collections::HashMap;

use nu_ansi_term::{Color, Style};
use nu_color_config::{get_color_config, get_color_map};
use nu_engine::CallExt;
use nu_explore::{run_pager, PagerConfig, StyleConfig};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

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
        let color_hm = get_color_config(nu_config);

        let mut config = nu_config.explore.clone();
        prepare_default_config(&mut config);

        if show_index {
            insert_bool(&mut config, "table_show_index", show_index);
        }

        if show_head {
            insert_bool(&mut config, "table_show_head", show_head);
        }

        let exit_esc = config
            .get("exit_esc")
            .and_then(|v| v.as_bool().ok())
            .unwrap_or(false);

        let style = style_from_config(&config);

        let mut config = PagerConfig::new(nu_config, &color_hm, &config);
        config.style = style;
        config.reverse = is_reverse;
        config.peek_value = peek_value;
        config.reverse = is_reverse;
        config.exit_esc = exit_esc;

        let result = run_pager(engine_state, stack, ctrlc, input, config);

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

fn style_from_config(config: &HashMap<String, Value>) -> StyleConfig {
    let mut style = StyleConfig::default();

    let colors = get_color_map(config);

    if let Some(s) = colors.get("status_bar") {
        style.status_bar = *s;
    }

    if let Some(s) = colors.get("command_bar") {
        style.cmd_bar = *s;
    }

    if let Some(s) = colors.get("highlight") {
        style.highlight = *s;
    }

    if let Some(s) = colors.get("status_info") {
        style.status_info = *s;
    }

    if let Some(s) = colors.get("status_warn") {
        style.status_warn = *s;
    }

    if let Some(s) = colors.get("status_error") {
        style.status_error = *s;
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

    insert_style(config, "status_bar", STATUS_BAR);
    insert_style(config, "command_bar", INPUT_BAR);
    insert_style(config, "highlight", HIGHLIGHT);
    insert_style(config, "status_info", STATUS_INFO);
    insert_style(config, "status_warn", STATUS_WARN);
    insert_style(config, "status_error", STATUS_ERROR);

    insert_style(config, "table_split_line", TABLE_SPLIT_LINE);
    insert_style(config, "table_selected_cell", TABLE_SELECT_CELL);
    insert_style(config, "table_selected_row", TABLE_SELECT_ROW);
    insert_style(config, "table_selected_column", TABLE_SELECT_COLUMN);
    insert_bool(config, "table_cursor", TABLE_SELECT_CURSOR);
    insert_bool(config, "table_line_head_top", TABLE_LINE_HEADER_TOP);
    insert_bool(config, "table_line_head_bottom", TABLE_LINE_HEADER_BOTTOM);
    insert_bool(config, "table_line_shift", TABLE_LINE_SHIFT);
    insert_bool(config, "table_line_index", TABLE_LINE_INDEX);
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

    if let Ok(val) = nu_json::to_string(&value) {
        map.insert(String::from(key), Value::string(val, Span::unknown()));
    }
}

fn insert_bool(map: &mut HashMap<String, Value>, key: &str, value: bool) {
    if map.contains_key(key) {
        return;
    }

    map.insert(String::from(key), Value::boolean(value, Span::unknown()));
}
