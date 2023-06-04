use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, 
    Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into duration"
    }

    fn signature(&self) -> Signature {
        Signature::build("into duration")
            .input_output_types(vec![
                (Type::String, Type::Duration),
                (Type::Duration, Type::Duration),
                // TODO: --convert option should be implemented as `format duration`
                (Type::String, Type::String),
                (Type::Duration, Type::String),
            ])
            .named(
                "convert",
                SyntaxShape::String,
                "convert duration into another duration",
                Some('c'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, convert data at the given cell paths",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to duration."
    }

    fn extra_usage(&self) -> &str {
        "This command does not take leap years into account, and every month is assumed to have 30 days."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "time", "period"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_duration(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let _span = Span::test_data();
        vec![]
    }
}

fn into_duration(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = match input.span() {
        Some(t) => t,
        None => call.head,
    };
    let convert_to_unit: Option<Spanned<String>> = call.get_flag(engine_state, stack, "convert")?;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let config = engine_state.get_config();
    let float_precision = config.float_precision as usize;

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, &convert_to_unit, float_precision, span)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let d = convert_to_unit.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, &d, float_precision, span)),
                    );
                    if let Err(error) = r {
                        return Value::Error {
                            error: Box::new(error),
                        };
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}



#[allow(unused_variables)]
fn action(
    input: &Value,
    convert_to_unit: &Option<Spanned<String>>,
    float_precision: usize,
    span: Span,
) -> Value {
    Value::Error {
        error: Box::new(ShellError::Unimplemented {
            desired_function: "into duration and all its subcommands".into(),
            span,
        }),
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
