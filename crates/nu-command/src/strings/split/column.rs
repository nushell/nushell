use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split column"
    }

    fn signature(&self) -> Signature {
        Signature::build("split column")
            .required(
                "separator",
                SyntaxShape::String,
                "the character that denotes what separates columns",
            )
            .switch("collapse-empty", "remove empty columns", Some('c'))
            .rest(
                "rest",
                SyntaxShape::String,
                "column names to give the new columns",
            )
    }

    fn usage(&self) -> &str {
        "splits contents across multiple columns via the separator."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        split_column(engine_state, stack, call, input)
    }
}

fn split_column(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let name_span = call.head;
    let separator: Spanned<String> = call.req(engine_state, stack, 0)?;
    let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 1)?;
    let collapse_empty = call.has_flag("collapse-empty");

    Ok(input
        .map(move |x| split_column_helper(&x, &separator, &rest, collapse_empty, name_span))
        .into_pipeline_data())
}

fn split_column_helper(
    v: &Value,
    separator: &Spanned<String>,
    rest: &[Spanned<String>],
    collapse_empty: bool,
    head: Span,
) -> Value {
    if let Ok(s) = v.as_string() {
        let splitter = separator.item.replace("\\n", "\n");

        let split_result: Vec<_> = if collapse_empty {
            s.split(&splitter).filter(|s| !s.is_empty()).collect()
        } else {
            s.split(&splitter).collect()
        };

        let positional: Vec<_> = rest.iter().map(|f| f.item.clone()).collect();

        // If they didn't provide column names, make up our own

        let mut cols = vec![];
        let mut vals = vec![];
        if positional.is_empty() {
            let mut gen_columns = vec![];
            for i in 0..split_result.len() {
                gen_columns.push(format!("Column{}", i + 1));
            }

            for (&k, v) in split_result.iter().zip(&gen_columns) {
                cols.push(v.to_string());
                vals.push(Value::string(k, head));
            }
        } else {
            for (&k, v) in split_result.iter().zip(&positional) {
                cols.push(v.into());
                vals.push(Value::string(k, head));
            }
        }
        Value::List {
            vals: vec![Value::Record {
                cols,
                vals,
                span: head,
            }],
            span: head,
        }
    } else {
        match v.span() {
            Ok(span) => Value::Error {
                error: ShellError::PipelineMismatch {
                    expected: Type::String,
                    expected_span: head,
                    origin: span,
                },
            },
            Err(error) => Value::Error { error },
        }
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
