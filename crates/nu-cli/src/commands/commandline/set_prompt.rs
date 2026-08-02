use nu_engine::command_prelude::*;
use nu_protocol::engine::PromptSegment;

#[derive(Clone)]
pub struct CommandlineSetPrompt;

impl Command for CommandlineSetPrompt {
    fn name(&self) -> &str {
        "commandline set-prompt"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::String, Type::Nothing),
            ])
            .optional(
                "prompt",
                SyntaxShape::String,
                "The rendered prompt text to display. If left-out, we read from pipeline input",
            )
            .named(
                "right",
                SyntaxShape::String,
                "Text for the right prompt.",
                Some('r'),
            )
            .named(
                "indicator",
                SyntaxShape::String,
                "Text for the prompt indicator.",
                Some('i'),
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Replace the current prompt and repaint it in place, without disturbing the line being edited."
    }

    fn extra_description(&self) -> &str {
        r#"This is meant to be called from a background job (see `job spawn`) to build
streaming prompts: we render the prompt as we know it up front, and each
`commandline set-prompt` updates our idea of what the prompt "is" for the
segments that have finished computing. The line and cursor are preserved.

`--indicator` sets the indicator for every edit mode at once (including the vi
insert and normal indicators), since there is a single indicator knob here.

The pushed prompt lasts only until the next prompt is drawn.

meant for REPL sessions only"#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["repl", "interactive", "async", "streaming", "repaint"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let right = call.get_flag::<String>(engine_state, stack, "right")?;
        let indicator = call.get_flag::<String>(engine_state, stack, "indicator")?;

        // Prefer the positional argument; fall back to the pipeline input so
        // both `commandline set-prompt $rendered` and `$rendered | commandline
        // set-prompt` work.
        let left = match call.opt::<String>(engine_state, stack, 0)? {
            Some(content) => Some(content),
            None => match input {
                PipelineData::Empty => None,
                input => Some(input.collect_string_strict(head)?.0),
            },
        };

        // Apply every provided segment under a single write lock and repaint once,
        // rather than locking and repainting per segment.
        if left.is_some() || right.is_some() || indicator.is_some() {
            engine_state.prompt_state.apply(|contents| {
                if let Some(content) = left {
                    contents.apply_segment_override(PromptSegment::Left, content);
                }
                if let Some(content) = right {
                    contents.apply_segment_override(PromptSegment::Right, content);
                }
                if let Some(content) = indicator {
                    contents.apply_segment_override(PromptSegment::Indicator, content);
                }
            });
        }

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"commandline set-prompt $"(ansi green)me> (ansi reset)""#,
                description: "Replace the left prompt with a freshly rendered string.",
                result: None,
            },
            Example {
                example: r#"commandline set-prompt --right $"right (date now | format date '%H:%M:%S')""#,
                description: "Replace the right prompt.",
                result: None,
            },
            Example {
                example: r#"commandline set-prompt --indicator $" (char prompt)""#,
                description: "Replace the indicator.",
                result: None,
            },
            Example {
                example: r#"commandline set-prompt --right "67" --indicator "69""#,
                description: "Replace multiple prompt segments in one call.",
                result: None,
            },
            Example {
                example: r#"job spawn { sleep 1sec; commandline set-prompt $"(git branch --show-current) > " }"#,
                description: "Stream a slow prompt segment in from a background job.",
                result: None,
            },
        ]
    }
}
