use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EvaluationContext},
    ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

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
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        split_row(context, call, input)
    }
}

fn split_row(
    context: &EvaluationContext,
    call: &Call,
    input: Value,
) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
    let name_span = call.head;
    let separator: Spanned<String> = call.req(context, 0)?;

    Ok(input.flat_map(name_span, move |x| {
        split_row_helper(&x, &separator, name_span)
    }))
}

fn split_row_helper(v: &Value, separator: &Spanned<String>, name: Span) -> Vec<Value> {
    if let Ok(s) = v.as_string() {
        let splitter = separator.item.replace("\\n", "\n");
        s.split(&splitter)
            .filter_map(|s| {
                if s.trim() != "" {
                    Some(Value::String {
                        val: s.into(),
                        span: v.span(),
                    })
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
                origin: v.span(),
            },
        }]
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
