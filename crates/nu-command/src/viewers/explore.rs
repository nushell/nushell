use std::collections::HashMap;

use nu_ansi_term::{Color, Style};
use nu_color_config::{get_color_config, get_color_map};
use nu_engine::CallExt;
use nu_explore::{StyleConfig, TableConfig, TableSplitLines, ViewConfig};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
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
        let table_cfg = TableConfig {
            show_index,
            show_head,
            peek_value,
            reverse: is_reverse,
            show_help: false,
        };

        let ctrlc = engine_state.ctrlc.clone();

        let config = engine_state.get_config();
        let color_hm = get_color_config(config);
        let style = theme_from_config(&config.explore);

        let view_cfg = ViewConfig::new(config, &color_hm, &style);

        let result = nu_explore::run_pager(engine_state, stack, ctrlc, table_cfg, view_cfg, input);

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

fn theme_from_config(config: &HashMap<String, Value>) -> StyleConfig {
    let colors = get_color_map(config);

    let mut style = default_style();

    if let Some(s) = colors.get("status_bar") {
        style.status_bar = *s;
    }

    if let Some(s) = colors.get("command_bar") {
        style.cmd_bar = *s;
    }

    if let Some(s) = colors.get("split_line") {
        style.split_line = *s;
    }

    if let Some(s) = colors.get("highlight") {
        style.highlight = *s;
    }

    if let Some(s) = colors.get("selected_cell") {
        style.selected_cell = Some(*s);
    }

    if let Some(s) = colors.get("selected_row") {
        style.selected_row = Some(*s);
    }

    if let Some(s) = colors.get("selected_column") {
        style.selected_column = Some(*s);
    }

    if let Some(show_cursor) = config.get("cursor").and_then(|v| v.as_bool().ok()) {
        style.show_cursow = show_cursor;
    }

    if let Some(b) = config.get("line_head_top").and_then(|v| v.as_bool().ok()) {
        style.split_lines.header_top = b;
    }

    if let Some(b) = config
        .get("line_head_bottom")
        .and_then(|v| v.as_bool().ok())
    {
        style.split_lines.header_bottom = b;
    }

    if let Some(b) = config.get("line_shift").and_then(|v| v.as_bool().ok()) {
        style.split_lines.shift_line = b;
    }

    if let Some(b) = config.get("line_index").and_then(|v| v.as_bool().ok()) {
        style.split_lines.index_line = b;
    }

    style
}

fn default_style() -> StyleConfig {
    StyleConfig {
        status_bar: Style {
            background: Some(Color::Rgb(196, 201, 198)),
            foreground: Some(Color::Rgb(29, 31, 33)),
            ..Default::default()
        },
        highlight: Style {
            background: Some(Color::Yellow),
            foreground: Some(Color::Black),
            ..Default::default()
        },
        split_line: Style {
            foreground: Some(Color::Rgb(64, 64, 64)),
            ..Default::default()
        },
        cmd_bar: Style {
            foreground: Some(Color::Rgb(196, 201, 198)),
            ..Default::default()
        },
        status_error: Style {
            background: Some(Color::Red),
            foreground: Some(Color::White),
            ..Default::default()
        },
        status_info: Style::default(),
        status_warn: Style::default(),
        selected_cell: None,
        selected_column: None,
        selected_row: None,
        show_cursow: true,
        split_lines: TableSplitLines {
            header_bottom: true,
            header_top: true,
            index_line: true,
            shift_line: true,
        },
    }
}
