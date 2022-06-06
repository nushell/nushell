use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Value};
use reedline::Highlighter;

#[derive(Clone)]
pub struct NuHighlight;

impl Command for NuHighlight {
    fn name(&self) -> &str {
        "nu-highlight"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu-highlight").category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Syntax highlight the input string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["syntax", "color", "convert"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();
        let config = engine_state.get_config().clone();

        let highlighter = crate::NuHighlighter {
            engine_state,
            config,
        };

        input.map(
            move |x| match x.as_string() {
                Ok(line) => {
                    let highlights = highlighter.highlight(&line, line.len());

                    Value::String {
                        val: highlights.render_simple(),
                        span: head,
                    }
                }
                Err(err) => Value::Error { error: err },
            },
            ctrlc,
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
