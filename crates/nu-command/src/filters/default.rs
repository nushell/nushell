use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Spanned, SyntaxShape, Value};

#[derive(Clone)]
pub struct Default;

impl Command for Default {
    fn name(&self) -> &str {
        "default"
    }

    fn signature(&self) -> Signature {
        Signature::build("default")
            .required("column name", SyntaxShape::String, "the name of the column")
            .required(
                "column value",
                SyntaxShape::Any,
                "the value of the column to default",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Sets a default row's column if missing."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        default(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Give a default 'target' to all file entries",
            example: "ls -la | default target 'nothing'",
            result: None,
        }]
    }
}

fn default(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let column: Spanned<String> = call.req(engine_state, stack, 0)?;
    let value: Value = call.req(engine_state, stack, 1)?;

    let ctrlc = engine_state.ctrlc.clone();

    input.map(
        move |item| match item {
            Value::Record {
                mut cols,
                mut vals,
                span,
            } => {
                let mut idx = 0;
                let mut found = false;

                while idx < cols.len() {
                    if cols[idx] == column.item && matches!(vals[idx], Value::Nothing { .. }) {
                        vals[idx] = value.clone();
                        found = true;
                    }
                    idx += 1;
                }

                if !found {
                    cols.push(column.item.clone());
                    vals.push(value.clone());
                }

                Value::Record { cols, vals, span }
            }
            _ => item,
        },
        ctrlc,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Default {})
    }
}
