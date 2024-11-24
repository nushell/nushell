use std::{
    io::{Read, Write},
    time::Duration,
};

use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TermQuery;

impl Command for TermQuery {
    fn name(&self) -> &str {
        "term query"
    }

    fn description(&self) -> &str {
        "Query the terminal for information."
    }

    fn extra_description(&self) -> &str {
        "Print the given query, and read the immediate result from stdin.

The standard input will be read right after `query` is printed, and consumed until the `terminator`
sequence is encountered. The `terminator` is not removed from the output."
    }

    fn signature(&self) -> Signature {
        Signature::build("term query")
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::Binary)])
            .allow_variants_without_examples(true)
            .required(
                "query",
                SyntaxShape::OneOf(vec![SyntaxShape::Binary, SyntaxShape::String]),
                "The query that will be printed to stdout.",
            )
            .required_named(
                "terminator",
                SyntaxShape::OneOf(vec![SyntaxShape::Binary, SyntaxShape::String]),
                "stdin will be read until this sequence is encountered.",
                Some('t'),
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let query: Vec<u8> = call.req(engine_state, stack, 0)?;
        let terminator: Vec<u8> = call
            .get_flag(engine_state, stack, "terminator")?
            .ok_or_else(|| ShellError::MissingParameter {
                param_name: "terminator".into(),
                span: call.head,
            })?;

        crossterm::terminal::enable_raw_mode()?;

        // clear terminal events
        while crossterm::event::poll(Duration::from_secs(0))? {
            // If there's an event, read it to remove it from the queue
            let _ = crossterm::event::read()?;
        }

        let mut b = [0u8; 1];
        let mut buf = vec![];
        let mut stdin = std::io::stdin().lock();

        {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&query)?;
            stdout.flush()?;
        }

        let out = loop {
            if let Err(err) = stdin.read_exact(&mut b) {
                break Err(ShellError::from(err));
            }

            if b[0] == 3 {
                break Err(ShellError::Interrupted { span: call.head });
            }

            buf.push(b[0]);

            if buf.ends_with(&terminator) {
                break Ok(Value::Binary {
                    val: buf,
                    internal_span: call.head,
                }
                .into_pipeline_data());
            }
        };
        crossterm::terminal::disable_raw_mode()?;
        out
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}
