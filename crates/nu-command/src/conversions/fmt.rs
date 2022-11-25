use crate::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct Fmt;

impl Command for Fmt {
    fn name(&self) -> &str {
        "fmt"
    }

    fn usage(&self) -> &str {
        "Format a number"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("fmt")
            .input_output_types(vec![(Type::Number, Type::Record(vec![]))])
            .category(Category::Conversions)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display", "render", "format"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get a record containing multiple formats for the number 42",
            example: "42 | fmt",
            result: Some(Value::Record {
                cols: vec![
                    "binary".into(),
                    "debug".into(),
                    "display".into(),
                    "lowerexp".into(),
                    "lowerhex".into(),
                    "octal".into(),
                    "upperexp".into(),
                    "upperhex".into(),
                ],
                vals: vec![
                    Value::String {
                        val: "0b101010".to_string(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "42".to_string(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "42".to_string(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "4.2e1".to_string(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "0x2a".to_string(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "0o52".to_string(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "4.2E1".to_string(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "0x2A".to_string(),
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        fmt(engine_state, stack, call, input)
    }
}

fn fmt(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let args = CellPathOnlyArgs::from(cell_paths);
    operate(action, args, input, call.head, engine_state.ctrlc.clone())
}

fn action(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    match input {
        Value::Int { val, .. } => fmt_it(*val, span),
        Value::Filesize { val, .. } => fmt_it(*val, span),
        _ => Value::Error {
            error: ShellError::UnsupportedInput(
                format!("unsupported input type: {:?}", input.get_type()),
                span,
            ),
        },
    }
}

fn fmt_it(num: i64, span: Span) -> Value {
    let mut cols = vec![];
    let mut vals = vec![];

    cols.push("binary".into());
    vals.push(Value::string(format!("{:#b}", num), span));

    cols.push("debug".into());
    vals.push(Value::string(format!("{:#?}", num), span));

    cols.push("display".into());
    vals.push(Value::string(format!("{}", num), span));

    cols.push("lowerexp".into());
    vals.push(Value::string(format!("{:#e}", num), span));

    cols.push("lowerhex".into());
    vals.push(Value::string(format!("{:#x}", num), span));

    cols.push("octal".into());
    vals.push(Value::string(format!("{:#o}", num), span));

    // cols.push("pointer".into());
    // vals.push(Value::string(format!("{:#p}", &num), span));

    cols.push("upperexp".into());
    vals.push(Value::string(format!("{:#E}", num), span));

    cols.push("upperhex".into());
    vals.push(Value::string(format!("{:#X}", num), span));

    Value::Record { cols, vals, span }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Fmt {})
    }
}
