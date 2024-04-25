use crate::{
    run_pager,
    util::{create_lscolors, create_map, map_into_value},
    PagerConfig, StyleConfig,
};
use nu_ansi_term::{Color, Style};
use nu_color_config::{get_color_map, StyleComputer};
use nu_engine::command_prelude::*;

use std::collections::HashMap;

/// A `less` like program to render a [`Value`] as a table.
#[derive(Clone)]
pub struct Explore;

impl Command for Explore {
    fn name(&self) -> &str {
        "explore"
    }

    fn usage(&self) -> &str {
        "Explore acts as a table pager, just like `less` does for text."
    }

    fn signature(&self) -> nu_protocol::Signature {
        // todo: Fix error message when it's empty
        // if we set h i short flags it panics????

        Signature::build("explore")
            .input_output_types(vec![(Type::Any, Type::Any)])
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
        r#"Press `:` then `h` to get a help menu."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let show_head: bool = call.get_flag(engine_state, stack, "head")?.unwrap_or(true);
        let show_index: bool = call.has_flag(engine_state, stack, "index")?;
        let is_reverse: bool = call.has_flag(engine_state, stack, "reverse")?;
        let peek_value: bool = call.has_flag(engine_state, stack, "peek")?;

        let ctrlc = engine_state.ctrlc.clone();
        let nu_config = engine_state.get_config();
        let style_computer = StyleComputer::from_config(engine_state, stack);

        let mut config = nu_config.explore.clone();
        include_nu_config(&mut config, &style_computer);
        update_config(&mut config, show_index, show_head);
        prepare_default_config(&mut config);

        let style = style_from_config(&config);

        let lscolors = create_lscolors(engine_state, stack);

        let mut config = PagerConfig::new(nu_config, &style_computer, &lscolors, config);
        config.style = style;
        config.reverse = is_reverse;
        config.peek_value = peek_value;
        config.reverse = is_reverse;

        let result = run_pager(engine_state, &mut stack.clone(), ctrlc, input, config);

        match result {
            Ok(Some(value)) => Ok(PipelineData::Value(value, None)),
            Ok(None) => Ok(PipelineData::Value(Value::default(), None)),
            Err(err) => Ok(PipelineData::Value(
                Value::error(err.into(), call.head),
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
                example: r#"glob *.md | each {|| open } | explore --index"#,
                result: None,
            },
            Example {
                description:
                    "Explore a JSON file, then save the last visited sub-structure to a file",
                example: r#"open file.json | explore --peek | to json | save part.json"#,
                result: None,
            },
        ]
    }
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

        if let Some(s) = colors.get("success") {
            style.status_success = *s;
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
    const STATUS_SUCCESS: Style = color(Some(Color::Black), Some(Color::Green));
    const STATUS_WARN: Style = color(None, None);

    const TABLE_SPLIT_LINE: Style = color(Some(Color::Rgb(64, 64, 64)), None);
    const TABLE_SELECT_CELL: Style = color(None, None);
    const TABLE_SELECT_ROW: Style = color(None, None);
    const TABLE_SELECT_COLUMN: Style = color(None, None);

    const HEXDUMP_INDEX: Style = color(Some(Color::Cyan), None);
    const HEXDUMP_SEGMENT: Style = color(Some(Color::Cyan), None).bold();
    const HEXDUMP_SEGMENT_ZERO: Style = color(Some(Color::Purple), None).bold();
    const HEXDUMP_SEGMENT_UNKNOWN: Style = color(Some(Color::Green), None).bold();
    const HEXDUMP_ASCII: Style = color(Some(Color::Cyan), None).bold();
    const HEXDUMP_ASCII_ZERO: Style = color(Some(Color::Purple), None).bold();
    const HEXDUMP_ASCII_UNKNOWN: Style = color(Some(Color::Green), None).bold();

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
        insert_style(&mut hm, "success", STATUS_SUCCESS);
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

        config.insert(String::from("table"), map_into_value(hm));
    }

    {
        let mut hm = config
            .get("hex-dump")
            .and_then(create_map)
            .unwrap_or_default();

        insert_style(&mut hm, "color_index", HEXDUMP_INDEX);
        insert_style(&mut hm, "color_segment", HEXDUMP_SEGMENT);
        insert_style(&mut hm, "color_segment_zero", HEXDUMP_SEGMENT_ZERO);
        insert_style(&mut hm, "color_segment_unknown", HEXDUMP_SEGMENT_UNKNOWN);
        insert_style(&mut hm, "color_ascii", HEXDUMP_ASCII);
        insert_style(&mut hm, "color_ascii_zero", HEXDUMP_ASCII_ZERO);
        insert_style(&mut hm, "color_ascii_unknown", HEXDUMP_ASCII_UNKNOWN);

        insert_int(&mut hm, "segment_size", 2);
        insert_int(&mut hm, "count_segments", 8);

        insert_bool(&mut hm, "split", true);

        config.insert(String::from("hex-dump"), map_into_value(hm));
    }
}

fn parse_hash_map(value: &Value) -> Option<HashMap<String, Value>> {
    value.as_record().ok().map(|val| {
        val.iter()
            .map(|(col, val)| (col.clone(), val.clone()))
            .collect::<HashMap<_, _>>()
    })
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
        prefix_with_reset: false,
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

    map.insert(String::from(key), Value::bool(value, Span::unknown()));
}

fn insert_int(map: &mut HashMap<String, Value>, key: &str, value: i64) {
    if map.contains_key(key) {
        return;
    }

    map.insert(String::from(key), Value::int(value, Span::unknown()));
}

fn include_nu_config(config: &mut HashMap<String, Value>, style_computer: &StyleComputer) {
    let line_color = lookup_color(style_computer, "separator");
    if line_color != nu_ansi_term::Style::default() {
        let mut map = config
            .get("table")
            .and_then(parse_hash_map)
            .unwrap_or_default();
        insert_style(&mut map, "split_line", line_color);
        config.insert(String::from("table"), map_into_value(map));
    }
}

fn lookup_color(style_computer: &StyleComputer, key: &str) -> nu_ansi_term::Style {
    style_computer.compute(key, &Value::nothing(Span::unknown()))
}
