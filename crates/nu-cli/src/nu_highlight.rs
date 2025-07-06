use std::sync::Arc;

use nu_engine::command_prelude::*;
use reedline::{Highlighter, StyledText};

use crate::syntax_highlight::highlight_syntax;

#[derive(Clone)]
pub struct NuHighlight;

impl Command for NuHighlight {
    fn name(&self) -> &str {
        "nu-highlight"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu-highlight")
            .category(Category::Strings)
            .switch(
                "reject-garbage",
                "Return an error if invalid syntax (garbage) was encountered",
                Some('r'),
            )
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
        let reject_garbage = call.has_flag(engine_state, stack, "reject-garbage")?;
        let head = call.head;

        let signals = engine_state.signals();

        let engine_state = Arc::new(engine_state.clone());
        let stack = Arc::new(stack.clone());

        input.map(
            move |x| match x.coerce_into_string() {
                Ok(line) => {
                    let result = highlight_syntax(&engine_state, &stack, &line, line.len());

                    let highlights = match (reject_garbage, result.found_garbage) {
                        (false, _) => result.text,
                        (true, None) => result.text,
                        (true, Some(span)) => {
                            let error = ShellError::OutsideSpannedLabeledError {
                                src: line,
                                error: "encountered invalid syntax while highlighting".into(),
                                msg: "invalid syntax".into(),
                                span,
                            };
                            return Value::error(error, head);
                        }
                    };

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
