use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    report_error_new, Category, Example, ListStream, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Echo;

impl Command for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn usage(&self) -> &str {
        "Returns its arguments, ignoring the piped-in value."
    }

    fn signature(&self) -> Signature {
        Signature::build("echo")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .rest("rest", SyntaxShape::Any, "the values to echo")
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"When given no arguments, it returns an empty string. When given one argument,
it returns it. Otherwise, it returns a list of the arguments. There is usually
little reason to use this over just writing the values as-is."#
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args = call.rest(engine_state, stack, 0);
        report_error_new(
                engine_state,
            &ShellError::GenericError(
                "Deprecated command".into(),
                "`echo` is deprecated and will be removed in 0.85.".into(),
                Some(call.head),
                Some("Please use the `print` command to print the data to the terminal or directly the value if you want to use it in a pipeline.".into()),
                vec![],
            ),
        );
        run(engine_state, args, call)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args = call.rest_const(working_set, 0);
        run(working_set.permanent(), args, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Put a list of numbers in the pipeline. This is the same as [1 2 3].",
                example: "echo 1 2 3",
                result: Some(Value::list(
                    vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                    Span::test_data(),
                )),
            },
            Example {
                description:
                    "Returns the piped-in value, by using the special $in variable to obtain it.",
                example: "echo $in",
                result: None,
            },
        ]
    }
}

fn run(
    engine_state: &EngineState,
    args: Result<Vec<Value>, ShellError>,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    args.map(|to_be_echoed| {
        let n = to_be_echoed.len();
        match n.cmp(&1usize) {
            //  More than one value is converted in a stream of values
            std::cmp::Ordering::Greater => PipelineData::ListStream(
                ListStream::from_stream(to_be_echoed.into_iter(), engine_state.ctrlc.clone()),
                None,
            ),

            //  But a single value can be forwarded as it is
            std::cmp::Ordering::Equal => PipelineData::Value(to_be_echoed[0].clone(), None),

            //  When there are no elements, we echo the empty string
            std::cmp::Ordering::Less => PipelineData::Value(Value::string("", call.head), None),
        }
    })
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Echo;
        use crate::test_examples;
        test_examples(Echo {})
    }
}
