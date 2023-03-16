use crate::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath, PathMember},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into cell-path"
    }

    fn signature(&self) -> Signature {
        Signature::build("into cell-path")
            .input_output_types(vec![(Type::String, Type::CellPath)])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .allow_variants_without_examples(true)
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to cell-path."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_cellpath(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Convert string into cell-path",
                example: "let s = 'b.b1'; $s | into cell-path | describe",
                result: Some(Value::String { val: "cell path".into(), span }),
            },
            Example {
                description: "Use a string as a cell-path",
                example: "let a = {a: 1, b:{b1:21, b2:22}}; let s = 'b.b1'; $a | get ($s | into cell-path)",
                result: Some(Value::Int { val: 21, span }),
            },
        ]
    }
}

fn into_cellpath(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let args = CellPathOnlyArgs::from(cell_paths);
    operate(action, args, input, call.head, engine_state.ctrlc.clone())
}

fn action(input: &Value, _hideargs: &CellPathOnlyArgs, span: Span) -> Value {
    match input {
        Value::String { val, .. } => {
            let pms: Vec<PathMember> = val
                .split(".")
                .map(|s| PathMember::String {
                    val: s.to_string(),
                    optional: false,
                    span,
                })
                .collect();
            Value::CellPath {
                val: CellPath { members: pms },
                span,
            }
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.expect_span(),
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
