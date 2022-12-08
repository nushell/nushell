use nu_protocol::ast::{CellPath, PathMember};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into cellpath"
    }

    fn signature(&self) -> Signature {
        Signature::build("into cellpath")
            .input_output_types(vec![(Type::String, Type::CellPath)])
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to a cellpath."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        into_cellpath(engine_state, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert from string to cellpath",
                example: " 'config.show_banner' | into cellpath",
                result: Some(Value::CellPath {
                    val: CellPath {
                        members: vec![
                            PathMember::String {
                                val: "config".to_string(),
                                span: Span::new(1, 21),
                            },
                            PathMember::String {
                                val: "show_banner".to_string(),
                                span: Span::new(1, 21),
                            },
                        ],
                    },
                    span: Span::new(1, 21),
                }),
            },
            Example {
                description: "Convert from string to cellpath",
                example: " 'a' | into cellpath",
                result: Some(Value::CellPath {
                    val: CellPath {
                        members: vec![PathMember::String {
                            val: "a".to_string(),
                            span: Span::new(38, 41),
                        }],
                    },
                    span: Span::new(1, 2),
                }),
            },
        ]
    }
}

fn into_cellpath(
    _: &EngineState,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let input = input.into_value(call.head);
    let res = match input {
        Value::String { val, span } => parse_string_into_cellapth(val, span),
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                "'into cellpath' does not support this input".into(),
                other.span().unwrap_or(call.head),
            ),
        },
    };
    Ok(res.into_pipeline_data())
}

fn parse_string_into_cellapth(val: String, span: Span) -> Value {
    let parts = val.split('.').collect::<Vec<&str>>();
    let mut cellpath: Vec<PathMember> = vec![];
    for part in parts {
        cellpath.push(PathMember::String {
            val: part.to_string(),
            span,
        })
    }
    Value::CellPath {
        val: CellPath { members: cellpath },
        span,
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
