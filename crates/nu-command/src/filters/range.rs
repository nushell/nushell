use nu_engine::command_prelude::*;
use nu_protocol::{report_parse_warning, ParseWarning};

#[derive(Clone)]
pub struct Range;

impl Command for Range {
    fn name(&self) -> &str {
        "range"
    }

    fn signature(&self) -> Signature {
        Signature::build("range")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required("rows", SyntaxShape::Range, "Range of rows to return.")
            .category(Category::Deprecated)
    }

    fn description(&self) -> &str {
        "Return only the selected rows."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["filter", "head", "tail", "slice"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[0,1,2,3,4,5] | range 4..5",
                description: "Get the last 2 items",
                result: Some(Value::list(
                    vec![Value::test_int(4), Value::test_int(5)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[0,1,2,3,4,5] | range (-2)..",
                description: "Get the last 2 items",
                result: Some(Value::list(
                    vec![Value::test_int(4), Value::test_int(5)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[0,1,2,3,4,5] | range (-3)..-2",
                description: "Get the next to last 2 items",
                result: Some(Value::list(
                    vec![Value::test_int(3), Value::test_int(4)],
                    Span::test_data(),
                )),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        report_parse_warning(
            &StateWorkingSet::new(engine_state),
            &ParseWarning::DeprecatedWarning {
                old_command: "range".into(),
                new_suggestion: "use `slice`".into(),
                span: head,
                url: "`help slice`".into(),
            },
        );
        super::Slice::run(&super::Slice, engine_state, stack, call, input)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Range {})
    }
}
