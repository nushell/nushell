use nu_engine::command_prelude::*;
use nu_protocol::{report_parse_warning, ParseWarning};

#[derive(Clone)]
pub struct Fmt;

impl Command for Fmt {
    fn name(&self) -> &str {
        "fmt"
    }

    fn description(&self) -> &str {
        "Format a number."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("fmt")
            .input_output_types(vec![(Type::Number, Type::record())])
            .category(Category::Deprecated)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get a record containing multiple formats for the number 42",
            example: "42 | fmt",
            result: Some(Value::test_record(record! {
                    "binary" =>   Value::test_string("0b101010"),
                    "debug" =>    Value::test_string("42"),
                    "display" =>  Value::test_string("42"),
                    "lowerexp" => Value::test_string("4.2e1"),
                    "lowerhex" => Value::test_string("0x2a"),
                    "octal" =>    Value::test_string("0o52"),
                    "upperexp" => Value::test_string("4.2E1"),
                    "upperhex" => Value::test_string("0x2A"),
            })),
        }]
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
                old_command: "fmt".into(),
                new_suggestion: "use `format number`".into(),
                span: head,
                url: "`help format number`".into(),
            },
        );
        crate::extra::strings::format::format_number(engine_state, stack, call, input)
    }
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
