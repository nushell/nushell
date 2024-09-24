use std::sync::Arc;

use nu_engine::command_prelude::*;
use reedline::{Highlighter, StyledText};

#[derive(Clone)]
pub struct NuHighlight;

impl Command for NuHighlight {
    fn name(&self) -> &str {
        "nu-highlight"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu-highlight")
            .category(Category::Strings)
            .input_output_types(vec![(Type::String, Type::String)])
    }

    fn description(&self) -> &str {
        "Syntax highlight the input string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["syntax", "color", "convert"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let signals = engine_state.signals();

        let highlighter = crate::NuHighlighter {
            engine_state: Arc::new(engine_state.clone()),
            stack: Arc::new(stack.clone()),
        };

        input.map(
            move |x| match x.coerce_into_string() {
                Ok(line) => {
                    let highlights = highlighter.highlight(&line, line.len());
                    Value::string(highlights.render_simple(), head)
                }
                Err(err) => Value::error(err, head),
            },
            signals,
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Describe the type of a string",
            example: "'let x = 3' | nu-highlight",
            result: None,
        }]
    }
}

/// A highlighter that does nothing
///
/// Used to remove highlighting from a reedline instance
/// (letting NuHighlighter structs be dropped)
#[derive(Default)]
pub struct NoOpHighlighter {}

impl Highlighter for NoOpHighlighter {
    fn highlight(&self, _line: &str, _cursor: usize) -> reedline::StyledText {
        StyledText::new()
    }
}
