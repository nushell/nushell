//! The explore command implementation.

use crate::explore::config::ExploreConfig;
use crate::explore::nu_common::create_lscolors;
use crate::explore::pager::PagerConfig;
use crate::explore::run_pager;
use nu_ansi_term::Style;
use nu_color_config::StyleComputer;
use nu_engine::command_prelude::*;

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
                "Show or hide column headers (default true).",
                None,
            )
            .switch("index", "Show row indexes when viewing a list.", Some('i'))
            .switch(
                "tail",
                "Start with the viewport scrolled to the bottom.",
                Some('t'),
            )
            .switch(
                "peek",
                "When quitting, output the value of the cell the cursor was on.",
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
            Ok(Some(value)) => Ok(PipelineData::value(value, None)),
            Ok(None) => Ok(PipelineData::value(Value::default(), None)),
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

                Ok(PipelineData::value(
                    Value::error(shell_error, call.head),
                    None,
                ))
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
                description: "Explore a JSON file, then save the last visited sub-structure to a file",
                example: r#"open file.json | explore --peek | to json | save part.json"#,
                result: None,
            },
        ]
    }
}

fn lookup_color(style_computer: &StyleComputer, key: &str) -> Style {
    style_computer.compute(key, &Value::nothing(Span::unknown()))
}
