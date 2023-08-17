use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};

#[derive(Clone)]
pub struct Fmt;

impl Command for Fmt {
    fn name(&self) -> &str {
        "fmt"
    }

    fn usage(&self) -> &str {
        "Format a number."
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
            result: Some(SpannedValue::Record {
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
                    SpannedValue::test_string("0b101010"),
                    SpannedValue::test_string("42"),
                    SpannedValue::test_string("42"),
                    SpannedValue::test_string("4.2e1"),
                    SpannedValue::test_string("0x2a"),
                    SpannedValue::test_string("0o52"),
                    SpannedValue::test_string("4.2E1"),
                    SpannedValue::test_string("0x2A"),
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
    ) -> Result<PipelineData, ShellError> {
        fmt(engine_state, stack, call, input)
    }
}

fn fmt(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let args = CellPathOnlyArgs::from(cell_paths);
    operate(action, args, input, call.head, engine_state.ctrlc.clone())
}

fn action(input: &SpannedValue, _args: &CellPathOnlyArgs, span: Span) -> SpannedValue {
    match input {
        SpannedValue::Float { val, .. } => fmt_it_64(*val, span),
        SpannedValue::Int { val, .. } => fmt_it(*val, span),
        SpannedValue::Filesize { val, .. } => fmt_it(*val, span),
        // Propagate errors by explicitly matching them before the final case.
        SpannedValue::Error { .. } => input.clone(),
        other => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "float , integer or filesize".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.expect_span(),
            }),
        },
    }
}

fn fmt_it(num: i64, span: Span) -> SpannedValue {
    let mut cols = vec![];
    let mut vals = vec![];

    cols.push("binary".into());
    vals.push(SpannedValue::string(format!("{num:#b}"), span));

    cols.push("debug".into());
    vals.push(SpannedValue::string(format!("{num:#?}"), span));

    cols.push("display".into());
    vals.push(SpannedValue::string(format!("{num}"), span));

    cols.push("lowerexp".into());
    vals.push(SpannedValue::string(format!("{num:#e}"), span));

    cols.push("lowerhex".into());
    vals.push(SpannedValue::string(format!("{num:#x}"), span));

    cols.push("octal".into());
    vals.push(SpannedValue::string(format!("{num:#o}"), span));

    // cols.push("pointer".into());
    // vals.push(Value::string(format!("{:#p}", &num), span));

    cols.push("upperexp".into());
    vals.push(SpannedValue::string(format!("{num:#E}"), span));

    cols.push("upperhex".into());
    vals.push(SpannedValue::string(format!("{num:#X}"), span));

    SpannedValue::Record { cols, vals, span }
}

fn fmt_it_64(num: f64, span: Span) -> SpannedValue {
    let mut cols = vec![];
    let mut vals = vec![];

    cols.push("binary".into());
    vals.push(SpannedValue::string(format!("{:b}", num.to_bits()), span));

    cols.push("debug".into());
    vals.push(SpannedValue::string(format!("{num:#?}"), span));

    cols.push("display".into());
    vals.push(SpannedValue::string(format!("{num}"), span));

    cols.push("lowerexp".into());
    vals.push(SpannedValue::string(format!("{num:#e}"), span));

    cols.push("lowerhex".into());
    vals.push(SpannedValue::string(format!("{:0x}", num.to_bits()), span));

    cols.push("octal".into());
    vals.push(SpannedValue::string(format!("{:0o}", num.to_bits()), span));

    // cols.push("pointer".into());
    // vals.push(Value::string(format!("{:#p}", &num), span));

    cols.push("upperexp".into());
    vals.push(SpannedValue::string(format!("{num:#E}"), span));

    cols.push("upperhex".into());
    vals.push(SpannedValue::string(format!("{:0X}", num.to_bits()), span));

    SpannedValue::Record { cols, vals, span }
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
