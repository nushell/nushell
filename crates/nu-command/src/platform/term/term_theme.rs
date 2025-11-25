use std::time::Duration;

use nu_engine::{CallExt, command_prelude::IoError};
use nu_protocol::{
    Category, Example, IntoPipelineData, ShellError, Signature, SyntaxShape, Type, Value,
    engine::Command, record,
};
use terminal_colorsaurus::{QueryOptions, ThemeMode, color_palette};

#[derive(Clone)]
pub struct TermTheme;

impl Command for TermTheme {
    fn name(&self) -> &str {
        "term theme"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("term theme")
            .category(Category::Platform)
            .input_output_type(
                Type::Nothing,
                Type::Record(
                    [
                        (
                            "background".into(),
                            Type::Record(
                                [
                                    ("r".into(), Type::Int),
                                    ("g".into(), Type::Int),
                                    ("b".into(), Type::Int),
                                ]
                                .into(),
                            ),
                        ),
                        (
                            "foreground".into(),
                            Type::Record(
                                [
                                    ("r".into(), Type::Int),
                                    ("g".into(), Type::Int),
                                    ("b".into(), Type::Int),
                                ]
                                .into(),
                            ),
                        ),
                        ("mode".into(), Type::String),
                    ]
                    .into(),
                ),
            )
            .named(
                "timeout",
                SyntaxShape::Duration,
                "Maximum time to wait for a response from the terminal (default is 1s)",
                Some('t'),
            )
    }

    fn description(&self) -> &str {
        "Query the terminal for the current background and foreground colors."
    }

    fn extra_description(&self) -> &str {
        "This queries the terminal using the `OSC 10` and `OSC 11` terminal sequences and returns a record containing the current background and foreground colors in 8bit RGB format.

Also returns whether this color scheme is considered light (dark text on light background) or dark (light text on dark background), based on the perceived lightness of the background and foreground colors."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "term theme",
                description: "Query the current terminal theme",
                result: Some(Value::test_record(record! {
                    "background" => Value::test_record(record! {
                        "r" => Value::test_int(239),
                        "g" => Value::test_int(241),
                        "b" => Value::test_int(245),
                    }),
                    "foreground" => Value::test_record(record! {
                        "r" => Value::test_int(76),
                        "g" => Value::test_int(79),
                        "b" => Value::test_int(105),
                    }),
                    "mode" => Value::test_string("light"),
                })),
            },
            Example {
                example: "term theme --timeout 2s",
                description: "Query the current terminal theme with a custom timeout (in case of high latency such as an SSH connection)",
                result: Some(Value::test_record(record! {
                    "background" => Value::test_record(record! {
                        "r" => Value::test_int(239),
                        "g" => Value::test_int(241),
                        "b" => Value::test_int(245),
                    }),
                    "foreground" => Value::test_record(record! {
                        "r" => Value::test_int(76),
                        "g" => Value::test_int(79),
                        "b" => Value::test_int(105),
                    }),
                    "mode" => Value::test_string("light"),
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        call: &nu_protocol::engine::Call,
        _input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let mut query_options = QueryOptions::default();
        if let Some(d) = call.get_flag::<Duration>(engine_state, stack, "timeout")? {
            query_options.timeout = d;
        }
        let palette = color_palette(query_options).map_err(|e| match &e {
            terminal_colorsaurus::Error::Io(io) => IoError::new(io, head, None).into(),
            terminal_colorsaurus::Error::Parse(_items) => ShellError::GenericError {
                error: "Terminal returned a color in an unsupported format".into(),
                msg: e.to_string(),
                span: Some(head),
                help: None,
                inner: vec![],
            },
            terminal_colorsaurus::Error::Timeout(_duration) => ShellError::GenericError {
                error: "Timeout expired".into(),
                msg: e.to_string(),
                span: Some(head),
                help: Some("If your terminal supports querying background and foreground colors, consider using a higher timeout value with `--timeout` in case there is a high latency.".into()),
                inner: vec![],
            },
            terminal_colorsaurus::Error::UnsupportedTerminal(err) => ShellError::GenericError {
                error: "Unsupported terminal".into(),
                msg: err.to_string(),
                span: Some(head),
                help: None,
                inner: vec![],
            },
            _ => ShellError::GenericError {
                error: "Unexpected error".into(),
                msg: e.to_string(),
                span: Some(head),
                help: None,
                inner: vec![],
            },
        })?;
        let (bg_r, bg_g, bg_b) = palette.background.scale_to_8bit();
        let (fg_r, fg_g, fg_b) = palette.foreground.scale_to_8bit();
        Ok(Value::record(
            record! {
                "background" => Value::record(record! {
                    "r" => Value::int(bg_r as i64, head),
                    "g" => Value::int(bg_g as i64, head),
                    "b" => Value::int(bg_b as i64, head),
                }, head),
                "foreground" => Value::record(record! {
                    "r" => Value::int(fg_r as i64, head),
                    "g" => Value::int(fg_g as i64, head),
                    "b" => Value::int(fg_b as i64, head),
                }, head),
                "mode" => match palette.theme_mode() {
                    ThemeMode::Dark => Value::string("dark", head),
                    ThemeMode::Light => Value::string("light", head),
                }
            },
            head,
        )
        .into_pipeline_data())
    }
}
