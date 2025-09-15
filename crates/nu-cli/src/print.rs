use nu_engine::command_prelude::*;
use nu_protocol::ByteStreamSource;

#[derive(Clone)]
pub struct Print;

impl Command for Print {
    fn name(&self) -> &str {
        "print"
    }

    fn signature(&self) -> Signature {
        Signature::build("print")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::Any, Type::Nothing),
            ])
            .allow_variants_without_examples(true)
            .rest("rest", SyntaxShape::Any, "the values to print")
            .switch(
                "no-newline",
                "print without inserting a newline for the line ending",
                Some('n'),
            )
            .switch("stderr", "print to stderr instead of stdout", Some('e'))
            .switch(
                "raw",
                "print without formatting (including binary data)",
                Some('r'),
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Print the given values to stdout."
    }

    fn extra_description(&self) -> &str {
        r#"Unlike `echo`, this command does not return any value (`print | describe` will return "nothing").
Since this command has no output, there is no point in piping it with other commands.

`print` may be used inside blocks of code (e.g.: hooks) to display text during execution without interfering with the pipeline."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let no_newline = call.has_flag(engine_state, stack, "no-newline")?;
        let raw = call.has_flag(engine_state, stack, "raw")?;

        // if we're in the LSP *always* print to stderr
        let to_stderr = if engine_state.is_lsp {
            true
        } else {
            call.has_flag(engine_state, stack, "stderr")?
        };

        // This will allow for easy printing of pipelines as well
        if !args.is_empty() {
            for arg in args {
                if raw {
                    arg.into_pipeline_data()
                        .print_raw(engine_state, no_newline, to_stderr)?;
                } else {
                    arg.into_pipeline_data().print_table(
                        engine_state,
                        stack,
                        no_newline,
                        to_stderr,
                    )?;
                }
            }
        } else if !input.is_nothing() {
            if let PipelineData::ByteStream(stream, _) = &mut input
                && let ByteStreamSource::Child(child) = stream.source_mut()
            {
                child.ignore_error(true);
            }
            if raw {
                input.print_raw(engine_state, no_newline, to_stderr)?;
            } else {
                input.print_table(engine_state, stack, no_newline, to_stderr)?;
            }
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Print 'hello world'",
                example: r#"print "hello world""#,
                result: None,
            },
            Example {
                description: "Print the sum of 2 and 3",
                example: r#"print (2 + 3)"#,
                result: None,
            },
            Example {
                description: "Print 'ABC' from binary data",
                example: r#"0x[41 42 43] | print --raw"#,
                result: None,
            },
        ]
    }
}
