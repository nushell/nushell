use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split row"
    }

    fn signature(&self) -> Signature {
        Signature::build("split row").required(
            "separator",
            SyntaxShape::String,
            "the character that denotes what separates rows",
        )
    }

    fn usage(&self) -> &str {
        "splits contents over multiple rows via the separator."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        split_row(engine_state, stack, call, input)
    }
}

fn split_row(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let name_span = call.head;
    let separator: Spanned<String> = call.req(engine_state, stack, 0)?;

    input.flat_map(
        move |x| split_row_helper(&x, &separator, name_span),
        engine_state.ctrlc.clone(),
    )
}

fn split_row_helper(v: &Value, separator: &Spanned<String>, name: Span) -> Vec<Value> {
    match v.span() {
        Ok(v_span) => {
            if let Ok(s) = v.as_string() {
                let splitter = separator.item.replace("\\n", "\n");
                s.split(&splitter)
                    .filter_map(|s| {
                        if s.trim() != "" {
                            Some(Value::string(s, v_span))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                vec![Value::Error {
                    error: ShellError::PipelineMismatch {
                        expected: Type::String,
                        expected_span: name,
                        origin: v_span,
                    },
                }]
            }
        }
        Err(error) => vec![Value::Error { error }],
    }
}

// #[cfg(test)]
// mod tests {
//     use super::ShellError;
//     use super::SubCommand;

//     #[test]
//     fn examples_work_as_expected() -> Result<(), ShellError> {
//         use crate::examples::test as test_examples;

//         test_examples(SubCommand {})
//     }
// }
