use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
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
        Signature::build("fmt").category(Category::Conversions)
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
                    Value::String("0b101010".to_string()),
                    Value::String("42".to_string()),
                    Value::String("42".to_string()),
                    Value::String("4.2e1".to_string()),
                    Value::String("0x2a".to_string()),
                    Value::String("0o52".to_string()),
                    Value::String("4.2E1".to_string()),
                    Value::String("0x2A".to_string()),
                ],
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
                        return Value::Error ( error );
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
        head,
    )
}

pub fn action(input: &Value, span: Span) -> Value {
    match input {
        Value::Int(val) => fmt_it(*val, span),
        Value::Filesize(val) => fmt_it(*val, span),
        _ => Value::Error(ShellError::UnsupportedInput(
            format!("unsupported input type: {:?}", input.get_type()),
            span,
        )),
    }
}

fn fmt_it(num: i64, span: Span) -> Value {
    let mut cols = vec![];
    let mut vals = vec![];

    cols.push("binary".into());
    vals.push(Value::string(format!("{:#b}", num)));

    cols.push("debug".into());
    vals.push(Value::string(format!("{:#?}", num)));

    cols.push("display".into());
    vals.push(Value::string(format!("{}", num)));

    cols.push("lowerexp".into());
    vals.push(Value::string(format!("{:#e}", num)));

    cols.push("lowerhex".into());
    vals.push(Value::string(format!("{:#x}", num)));

    cols.push("octal".into());
    vals.push(Value::string(format!("{:#o}", num)));

    // cols.push("pointer".into());
    // vals.push(Value::string(format!("{:#p}", &num), ));

    cols.push("upperexp".into());
    vals.push(Value::string(format!("{:#E}", num)));

    cols.push("upperhex".into());
    vals.push(Value::string(format!("{:#X}", num)));

    Value::Record { cols, vals }
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
