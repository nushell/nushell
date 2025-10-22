use std::sync::Arc;

use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::Config;

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
    config: Arc<Config>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct AnsiStrip;

impl Command for AnsiStrip {
    fn name(&self) -> &str {
        "ansi strip"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi strip")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::String))),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .rest(
                "cell path",
                SyntaxShape::CellPath,
                "For a data structure input, remove ANSI sequences from strings at the given cell paths.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Strip ANSI escape sequences from a string."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let config = stack.get_config(engine_state);
        let args = Arguments { cell_paths, config };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Strip ANSI escape sequences from a string",
                example: r#"$'(ansi green)(ansi cursor_on)hello' | ansi strip"#,
                result: Some(Value::test_string("hello")),
            },
            Example {
                description: "Strip ANSI escape sequences from a record field",
                example: r#"{ greeting: $'hello (ansi red)world' exclamation: false } | ansi strip greeting"#,
                result: Some(Value::test_record(record! {
                    "greeting" => Value::test_string("hello world"),
                    "exclamation" => Value::test_bool(false)
                })),
            },
            Example {
                description: "Strip ANSI escape sequences from multiple table columns",
                example: r#"[[language feature]; [$'(ansi red)rust' $'(ansi i)safety']] | ansi strip language feature"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "language" => Value::test_string("rust"),
                    "feature" => Value::test_string("safety")
                })])),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, _span: Span) -> Value {
    let span = input.span();
    match input {
        Value::String { val, .. } => {
            Value::string(nu_utils::strip_ansi_likely(val).to_string(), span)
        }
        other => {
            // Fake stripping ansi for other types and just show the abbreviated string
            // instead of showing an error message
            Value::string(other.to_abbreviated_string(&args.config), span)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AnsiStrip, Arguments, action};
    use nu_protocol::{Span, Value, engine::EngineState};

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;

        test_examples(AnsiStrip {})
    }

    #[test]
    fn test_stripping() {
        let input_string =
            Value::test_string("\u{1b}[3;93;41mHello\u{1b}[0m \u{1b}[1;32mNu \u{1b}[1;35mWorld");
        let expected = Value::test_string("Hello Nu World");

        let args = Arguments {
            cell_paths: vec![].into(),
            config: EngineState::new().get_config().clone(),
        };

        let actual = action(&input_string, &args, Span::test_data());
        assert_eq!(actual, expected);
    }
}
