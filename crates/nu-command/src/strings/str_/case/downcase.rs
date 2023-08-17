use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{
    Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str downcase"
    }

    fn signature(&self) -> Signature {
        Signature::build("str downcase")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert strings at the given cell paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Make text lowercase."
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
                result: Some(SpannedValue::test_string("nu")),
            },
            Example {
                description: "Downcase contents",
                example: "'TESTa' | str downcase",
                result: Some(SpannedValue::test_string("testa")),
            },
            Example {
                description: "Downcase contents",
                example: "[[ColA ColB]; [Test ABC]] | str downcase ColA",
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string()],
                        vals: vec![
                            SpannedValue::test_string("test"),
                            SpannedValue::test_string("ABC"),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Downcase contents",
                example: "[[ColA ColB]; [Test ABC]] | str downcase ColA ColB",
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string()],
                        vals: vec![
                            SpannedValue::test_string("test"),
                            SpannedValue::test_string("abc"),
                        ],
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
                        return SpannedValue::Error {
                            error: Box::new(error),
                        };
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn action(input: &SpannedValue, head: Span) -> SpannedValue {
    match input {
        SpannedValue::String { val, .. } => SpannedValue::String {
            val: val.to_ascii_lowercase(),
            span: head,
        },
        SpannedValue::Error { .. } => input.clone(),
        _ => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.expect_span(),
            }),
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
