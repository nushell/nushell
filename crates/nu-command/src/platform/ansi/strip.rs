use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call, ast::CellPath, engine::Command, engine::EngineState, engine::Stack, Category,
    Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "ansi strip"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi strip")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::String))),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .rest(
                "cell path",
                SyntaxShape::CellPath,
                "for a data structure input, remove ANSI sequences from strings at the given cell paths",
            )
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn usage(&self) -> &str {
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
        let arg = CellPathOnlyArgs::from(cell_paths);
        operate(action, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Strip ANSI escape sequences from a string",
            example: r#"$'(ansi green)(ansi cursor_on)hello' | ansi strip"#,
            result: Some(SpannedValue::test_string("hello")),
        }]
    }
}

fn action(input: &SpannedValue, _args: &CellPathOnlyArgs, command_span: Span) -> SpannedValue {
    match input {
        SpannedValue::String { val, span } => {
            SpannedValue::string(nu_utils::strip_ansi_likely(val).to_string(), *span)
        }
        other => {
            let got = format!("value is {}, not string", other.get_type());

            SpannedValue::Error {
                error: Box::new(ShellError::TypeMismatch {
                    err_message: got,
                    span: other.span().unwrap_or(command_span),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{action, SubCommand};
    use nu_protocol::{Span, SpannedValue};

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn test_stripping() {
        let input_string = SpannedValue::test_string(
            "\u{1b}[3;93;41mHello\u{1b}[0m \u{1b}[1;32mNu \u{1b}[1;35mWorld",
        );
        let expected = SpannedValue::test_string("Hello Nu World");

        let actual = action(&input_string, &vec![].into(), Span::test_data());
        assert_eq!(actual, expected);
    }
}
