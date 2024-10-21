use crate::{
    run_pager,
    util::{create_lscolors, create_map},
    PagerConfig,
};
use nu_ansi_term::{Color, Style};
use nu_color_config::{get_color_map, StyleComputer};
use nu_engine::command_prelude::*;
use nu_protocol::Config;

/// A `less` like program to render a [`Value`] as a table.
#[derive(Clone)]
pub struct Explore;

impl Command for Explore {
    fn name(&self) -> &str {
        "explore"
    }

    fn description(&self) -> &str {
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
                "tail",
                "Start with the viewport scrolled to the bottom",
                Some('t'),
            )
            .switch(
                "peek",
                "When quitting, output the value of the cell the cursor was on",
                Some('p'),
            )
            .category(Category::Viewers)
    }

    fn extra_description(&self) -> &str {
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
        let tail: bool = call.has_flag(engine_state, stack, "tail")?;
        let peek_value: bool = call.has_flag(engine_state, stack, "peek")?;

        let nu_config = stack.get_config(engine_state);
        let style_computer = StyleComputer::from_config(engine_state, stack);

        let mut explore_config = ExploreConfig::from_nu_config(&nu_config);
        explore_config.table.show_header = show_head;
        explore_config.table.show_index = show_index;
        explore_config.table.separator_style = lookup_color(&style_computer, "separator");

        let lscolors = create_lscolors(engine_state, stack);
        let cwd = engine_state.cwd(Some(stack)).map_or(String::new(), |path| {
            path.to_str().unwrap_or("").to_string()
        });

        let config = PagerConfig::new(
            &nu_config,
            &explore_config,
            &style_computer,
            &lscolors,
            peek_value,
            tail,
            &cwd,
        );

        let result = run_pager(engine_state, &mut stack.clone(), input, config);

        match result {
            Ok(Some(value)) => Ok(PipelineData::Value(value, None)),
            Ok(None) => Ok(PipelineData::Value(Value::default(), None)),
            Err(err) => {
                let shell_error = match err.downcast::<ShellError>() {
                    Ok(e) => e,
                    Err(e) => ShellError::GenericError {
                        error: e.to_string(),
                        msg: "".into(),
                        span: None,
                        help: None,
                        inner: vec![],
                    },
                };

                Ok(PipelineData::Value(
                    Value::error(shell_error, call.head),
                    None,
                ))
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Explore the system host information record",
                example: r#"sys host | explore"#,
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

#[derive(Debug, Clone)]
pub struct ExploreConfig {
    pub table: TableConfig,
    pub selected_cell: Style,
    pub status_info: Style,
    pub status_success: Style,
    pub status_warn: Style,
    pub status_error: Style,
    pub status_bar_background: Style,
    pub status_bar_text: Style,
    pub cmd_bar_text: Style,
    pub cmd_bar_background: Style,
    pub highlight: Style,
    /// if true, the explore view will immediately try to run the command as it is typed
    pub try_reactive: bool,
}

impl Default for ExploreConfig {
    fn default() -> Self {
        Self {
            table: TableConfig::default(),
            selected_cell: color(None, Some(Color::LightBlue)),
            status_info: color(None, None),
            status_success: color(Some(Color::Black), Some(Color::Green)),
            status_warn: color(None, None),
            status_error: color(Some(Color::White), Some(Color::Red)),
            status_bar_background: color(
                Some(Color::Rgb(29, 31, 33)),
                Some(Color::Rgb(196, 201, 198)),
            ),
            status_bar_text: color(None, None),
            cmd_bar_text: color(Some(Color::Rgb(196, 201, 198)), None),
            cmd_bar_background: color(None, None),
            highlight: color(Some(Color::Black), Some(Color::Yellow)),
            try_reactive: false,
        }
    }
}
impl ExploreConfig {
    /// take the default explore config and update it with relevant values from the nu config
    pub fn from_nu_config(config: &Config) -> Self {
        let mut ret = Self::default();

        ret.table.column_padding_left = config.table.padding.left;
        ret.table.column_padding_right = config.table.padding.right;

        let explore_cfg_hash_map = config.explore.clone();
        let colors = get_color_map(&explore_cfg_hash_map);

        if let Some(s) = colors.get("status_bar_text") {
            ret.status_bar_text = *s;
        }

        if let Some(s) = colors.get("status_bar_background") {
            ret.status_bar_background = *s;
        }

        if let Some(s) = colors.get("command_bar_text") {
            ret.cmd_bar_text = *s;
        }

        if let Some(s) = colors.get("command_bar_background") {
            ret.cmd_bar_background = *s;
        }

        if let Some(s) = colors.get("command_bar_background") {
            ret.cmd_bar_background = *s;
        }

        if let Some(s) = colors.get("selected_cell") {
            ret.selected_cell = *s;
        }

        if let Some(hm) = explore_cfg_hash_map.get("status").and_then(create_map) {
            let colors = get_color_map(&hm);

            if let Some(s) = colors.get("info") {
                ret.status_info = *s;
            }

            if let Some(s) = colors.get("success") {
                ret.status_success = *s;
            }

            if let Some(s) = colors.get("warn") {
                ret.status_warn = *s;
            }

            if let Some(s) = colors.get("error") {
                ret.status_error = *s;
            }
        }

        if let Some(hm) = explore_cfg_hash_map.get("try").and_then(create_map) {
            if let Some(reactive) = hm.get("reactive") {
                if let Ok(b) = reactive.as_bool() {
                    ret.try_reactive = b;
                }
            }
        }

        ret
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TableConfig {
    pub separator_style: Style,
    pub show_index: bool,
    pub show_header: bool,
    pub column_padding_left: usize,
    pub column_padding_right: usize,
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

fn lookup_color(style_computer: &StyleComputer, key: &str) -> nu_ansi_term::Style {
    style_computer.compute(key, &Value::nothing(Span::unknown()))
}
