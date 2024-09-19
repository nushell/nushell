use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct HelpEscapes;

impl Command for HelpEscapes {
    fn name(&self) -> &str {
        "help escapes"
    }

    fn description(&self) -> &str {
        "Show help on nushell string escapes."
    }

    fn signature(&self) -> Signature {
        Signature::build("help escapes")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let escape_info = generate_escape_info();
        let mut recs = vec![];

        for escape in escape_info {
            recs.push(Value::record(
                record! {
                    "sequence" => Value::string(escape.sequence, head),
                    "output" => Value::string(escape.output, head),
                },
                head,
            ));
        }

        Ok(Value::list(recs, call.head).into_pipeline_data())
    }
}

struct EscapeInfo {
    sequence: String,
    output: String,
}

fn generate_escape_info() -> Vec<EscapeInfo> {
    vec![
        EscapeInfo {
            sequence: "\\\"".into(),
            output: "\"".into(),
        },
        EscapeInfo {
            sequence: "\\\'".into(),
            output: "\'".into(),
        },
        EscapeInfo {
            sequence: "\\\\".into(),
            output: "\\".into(),
        },
        EscapeInfo {
            sequence: "\\/".into(),
            output: "/".into(),
        },
        EscapeInfo {
            sequence: "\\(".into(),
            output: "(".into(),
        },
        EscapeInfo {
            sequence: "\\)".into(),
            output: ")".into(),
        },
        EscapeInfo {
            sequence: "\\{".into(),
            output: "{".into(),
        },
        EscapeInfo {
            sequence: "\\}".into(),
            output: "}".into(),
        },
        EscapeInfo {
            sequence: "\\$".into(),
            output: "$".into(),
        },
        EscapeInfo {
            sequence: "\\^".into(),
            output: "^".into(),
        },
        EscapeInfo {
            sequence: "\\#".into(),
            output: "#".into(),
        },
        EscapeInfo {
            sequence: "\\|".into(),
            output: "|".into(),
        },
        EscapeInfo {
            sequence: "\\~".into(),
            output: "~".into(),
        },
        EscapeInfo {
            sequence: "\\a".into(),
            output: "alert bell".into(),
        },
        EscapeInfo {
            sequence: "\\b".into(),
            output: "backspace".into(),
        },
        EscapeInfo {
            sequence: "\\e".into(),
            output: "escape".into(),
        },
        EscapeInfo {
            sequence: "\\f".into(),
            output: "form feed".into(),
        },
        EscapeInfo {
            sequence: "\\n".into(),
            output: "newline (line feed)".into(),
        },
        EscapeInfo {
            sequence: "\\r".into(),
            output: "carriage return".into(),
        },
        EscapeInfo {
            sequence: "\\t".into(),
            output: "tab".into(),
        },
        EscapeInfo {
            sequence: "\\u{X...}".into(),
            output: "a single unicode character, where X... is 1-6 digits (0-9, A-F)".into(),
        },
    ]
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::HelpEscapes;
        use crate::test_examples;
        test_examples(HelpEscapes {})
    }
}
