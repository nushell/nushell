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
        Ok(ESCAPE_INFO
            .iter()
            .map(|&info| info.into_value(head))
            .collect::<List>()
            .into_value(head)
            .into_pipeline_data())
    }
}

#[derive(Clone, Copy, IntoValue)]
struct EscapeInfo {
    sequence: &'static str,
    output: &'static str,
}

const ESCAPE_INFO: &[EscapeInfo] = &[
    EscapeInfo {
        sequence: "\\\"",
        output: "\"",
    },
    EscapeInfo {
        sequence: "\\\'",
        output: "\'",
    },
    EscapeInfo {
        sequence: "\\\\",
        output: "\\",
    },
    EscapeInfo {
        sequence: "\\/",
        output: "/",
    },
    EscapeInfo {
        sequence: "\\(",
        output: "(",
    },
    EscapeInfo {
        sequence: "\\)",
        output: ")",
    },
    EscapeInfo {
        sequence: "\\{",
        output: "{",
    },
    EscapeInfo {
        sequence: "\\}",
        output: "}",
    },
    EscapeInfo {
        sequence: "\\$",
        output: "$",
    },
    EscapeInfo {
        sequence: "\\^",
        output: "^",
    },
    EscapeInfo {
        sequence: "\\#",
        output: "#",
    },
    EscapeInfo {
        sequence: "\\|",
        output: "|",
    },
    EscapeInfo {
        sequence: "\\~",
        output: "~",
    },
    EscapeInfo {
        sequence: "\\a",
        output: "alert bell",
    },
    EscapeInfo {
        sequence: "\\b",
        output: "backspace",
    },
    EscapeInfo {
        sequence: "\\e",
        output: "escape",
    },
    EscapeInfo {
        sequence: "\\f",
        output: "form feed",
    },
    EscapeInfo {
        sequence: "\\n",
        output: "newline (line feed)",
    },
    EscapeInfo {
        sequence: "\\r",
        output: "carriage return",
    },
    EscapeInfo {
        sequence: "\\t",
        output: "tab",
    },
    EscapeInfo {
        sequence: "\\u{X...}",
        output: "a single unicode character, where X... is 1-6 digits (0-9, A-F)",
    },
];

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::HelpEscapes;
        use crate::test_examples;
        test_examples(HelpEscapes {})
    }
}
