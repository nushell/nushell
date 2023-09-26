use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    record, Category, PipelineData, IntoInterruptiblePipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct HelpEscapes;

impl Command for HelpEscapes {
    fn name(&self) -> &str {
        "help escapes"
    }

    fn usage(&self) -> &str {
        "Show help on nushell escapes."
    }

    fn signature(&self) -> Signature {
        Signature::build("help escapes")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
            .allow_variants_without_examples(true)
    }

    fn run(
        &self,
        engine_state: &EngineState,
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
                    "character" => Value::string(escape.character, head),
                },
                head,
            ));
        }

        Ok(recs
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()))
    }
}

struct EscapeInfo {
    character: String,
}

fn generate_escape_info() -> Vec<EscapeInfo> {
    vec![
        EscapeInfo {
            character: "\"".into(),
        },
        EscapeInfo {
            character: "\'".into(),
        },
        EscapeInfo {
            character: "\\".into(),
        },
        EscapeInfo {
            character: "/".into(),
        },
        EscapeInfo {
            character: "(".into(),
        },
        EscapeInfo {
            character: ")".into(),
        },
        EscapeInfo {
            character: "{".into(),
        },
        EscapeInfo {
            character: "}".into(),
        },
        EscapeInfo {
            character: "$".into(),
        },
        EscapeInfo {
            character: "^".into(),
        },
        EscapeInfo {
            character: "#".into(),
        },
        EscapeInfo {
            character: "|".into(),
        },
        EscapeInfo {
            character: "~".into(),
        },
        EscapeInfo {
            character: "a".into(),
        },
        EscapeInfo {
            character: "b".into(),
        },
        EscapeInfo {
            character: "e".into(),
        },
        EscapeInfo {
            character: "f".into(),
        },
        EscapeInfo {
            character: "n".into(),
        },
        EscapeInfo {
            character: "r".into(),
        },
        EscapeInfo {
            character: "t".into(),
        },
        EscapeInfo {
            character: "u".into(),
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
