use nu_engine::command_prelude::*;

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
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Print the given values to stdout."
    }

    fn extra_usage(&self) -> &str {
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
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let no_newline = call.has_flag(engine_state, stack, "no-newline")?;
        let to_stderr = call.has_flag(engine_state, stack, "stderr")?;

        // This will allow for easy printing of pipelines as well
        if !args.is_empty() {
            for arg in args {
                arg.into_pipeline_data()
                    .print(engine_state, stack, no_newline, to_stderr)?;
            }
        } else if !input.is_nothing() {
            input.print(engine_state, stack, no_newline, to_stderr)?;
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
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
        ]
    }
}
