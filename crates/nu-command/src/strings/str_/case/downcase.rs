use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str downcase"
    }

    fn signature(&self) -> Signature {
        Signature::build("str downcase")
            .input_output_types(vec![(Type::String, Type::String)])
            .vectorizes_over_list(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert strings at the given cell paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Make text lowercase"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["lower case", "lowercase"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Downcase contents",
                example: "'NU' | str downcase",
                result: Some(Value::test_string("nu")),
            },
            Example {
                description: "Downcase contents",
                example: "'TESTa' | str downcase",
                result: Some(Value::test_string("testa")),
            },
            Example {
                description: "Downcase contents",
                example: "[[ColA ColB]; [Test ABC]] | str downcase ColA",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string()],
                        vals: vec![Value::test_string("test"), Value::test_string("ABC")],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Downcase contents",
                example: "[[ColA ColB]; [Test ABC]] | str downcase ColA ColB",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string()],
                        vals: vec![Value::test_string("test"), Value::test_string("abc")],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r =
                        ret.update_cell_path(&path.members, Box::new(move |old| action(old, head)));
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn action(input: &Value, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::String {
            val: val.to_ascii_lowercase(),
            span: head,
        },
        Value::Error { .. } => input.clone(),
        _ => Value::Error {
            error: ShellError::OnlySupportsThisInputType(
                "string".into(),
                input.get_type().to_string(),
                head,
                input.expect_span(),
            ),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
