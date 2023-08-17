use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into bool"
    }

    fn signature(&self) -> Signature {
        Signature::build("into bool")
            .input_output_types(vec![
                (Type::Int, Type::Bool),
                (Type::Number, Type::Bool),
                (Type::String, Type::Bool),
                (Type::Bool, Type::Bool),
                (Type::List(Box::new(Type::Any)), Type::Table(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to boolean."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "boolean", "true", "false", "1", "0"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_bool(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Convert value to boolean in table",
                example: "[[value]; ['false'] ['1'] [0] [1.0] [true]] | into bool value",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![SpannedValue::bool(false, span)],
                            span,
                        },
                        SpannedValue::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![SpannedValue::bool(true, span)],
                            span,
                        },
                        SpannedValue::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![SpannedValue::bool(false, span)],
                            span,
                        },
                        SpannedValue::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![SpannedValue::bool(true, span)],
                            span,
                        },
                        SpannedValue::Record {
                            cols: vec!["value".to_string()],
                            vals: vec![SpannedValue::bool(true, span)],
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description: "Convert bool to boolean",
                example: "true | into bool",
                result: Some(SpannedValue::bool(true, span)),
            },
            Example {
                description: "convert integer to boolean",
                example: "1 | into bool",
                result: Some(SpannedValue::bool(true, span)),
            },
            Example {
                description: "convert decimal to boolean",
                example: "0.3 | into bool",
                result: Some(SpannedValue::bool(true, span)),
            },
            Example {
                description: "convert decimal string to boolean",
                example: "'0.0' | into bool",
                result: Some(SpannedValue::bool(false, span)),
            },
            Example {
                description: "convert string to boolean",
                example: "'true' | into bool",
                result: Some(SpannedValue::bool(true, span)),
            },
        ]
    }
}

fn into_bool(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let args = CellPathOnlyArgs::from(cell_paths);
    operate(action, args, input, call.head, engine_state.ctrlc.clone())
}

fn string_to_boolean(s: &str, span: Span) -> Result<bool, ShellError> {
    match s.trim().to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        o => {
            let val = o.parse::<f64>();
            match val {
                Ok(f) => Ok(f.abs() >= f64::EPSILON),
                Err(_) => Err(ShellError::CantConvert {
                    to_type: "boolean".to_string(),
                    from_type: "string".to_string(),
                    span,
                    help: Some(
                        r#"the strings "true" and "false" can be converted into a bool"#
                            .to_string(),
                    ),
                }),
            }
        }
    }
}

fn action(input: &SpannedValue, _args: &CellPathOnlyArgs, span: Span) -> SpannedValue {
    match input {
        SpannedValue::Bool { .. } => input.clone(),
        SpannedValue::Int { val, .. } => SpannedValue::Bool {
            val: *val != 0,
            span,
        },
        SpannedValue::Float { val, .. } => SpannedValue::Bool {
            val: val.abs() >= f64::EPSILON,
            span,
        },
        SpannedValue::String { val, .. } => match string_to_boolean(val, span) {
            Ok(val) => SpannedValue::Bool { val, span },
            Err(error) => SpannedValue::Error {
                error: Box::new(error),
            },
        },
        // Propagate errors by explicitly matching them before the final case.
        SpannedValue::Error { .. } => input.clone(),
        other => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "bool, integer, float or string".into(),
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
