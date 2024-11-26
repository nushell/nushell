use std::{
    io::{Read, Write},
    time::Duration,
};

use nu_engine::command_prelude::*;

const CTRL_C: u8 = 3;

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
sequence is encountered. The `terminator` is not included in the output.

If `terminator` is not supplied, input will be read until Ctrl-C is pressed.

If `beginning` is supplied, input's beginning will be validated against it.
The `beginning` is not included in the output."
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
            .named(
                "beginning",
                SyntaxShape::OneOf(vec![SyntaxShape::Binary, SyntaxShape::String]),
                "Beginning sequence for the expected reply.",
                Some('b'),
            )
            .named(
                "terminator",
                SyntaxShape::OneOf(vec![SyntaxShape::Binary, SyntaxShape::String]),
                "Terminator sequence for the expected reply.",
                Some('t'),
            )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get cursor position.",
                example: r#"term query (ansi cursor_position) --beginning (ansi csi) --terminator 'R'"#,
                result: None,
            },
            Example {
                description: "Get terminal background color.",
                example: r#"term query $'(ansi osc)10;?(ansi st)' --beginning $'(ansi osc)10;' --terminator (ansi st)"#,
                result: None,
            },
            Example {
                description: "Read clipboard content on terminals supporting OSC-52.",
                example: r#"term query $'(ansi osc)52;c;?(ansi st)' --beginning $'(ansi osc)52;c;' --terminator (ansi st)"#,
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let query: Vec<u8> = call.req(engine_state, stack, 0)?;
        let beginning: Option<Vec<u8>> = call.get_flag(engine_state, stack, "beginning")?;
        let beginning = beginning.unwrap_or_default();
        let terminator: Option<Vec<u8>> = call.get_flag(engine_state, stack, "terminator")?;

        crossterm::terminal::enable_raw_mode()?;
        scopeguard::defer! {
            let _ = crossterm::terminal::disable_raw_mode();
        }

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

        // Validate and skip beginning
        for bc in beginning {
            stdin.read_exact(&mut b)?;
            if b[0] != bc {
                return Err(ShellError::GenericError {
                    error: "Input did not begin with expected sequence".into(),
                    msg: "".into(),
                    span: None,
                    help: Some(
                        "Try running without `--beginning` and inspecting the output.".into(),
                    ),
                    inner: vec![],
                });
            }
        }

        if let Some(terminator) = terminator {
            loop {
                stdin.read_exact(&mut b)?;

                if b[0] == CTRL_C {
                    return Err(ShellError::InterruptedByUser {
                        span: Some(call.head),
                    });
                }

                buf.push(b[0]);

                if buf.ends_with(&terminator) {
                    // Remove terminator
                    buf.drain((buf.len() - terminator.len())..);
                    break;
                }
            }
        } else {
            loop {
                stdin.read_exact(&mut b)?;

                if b[0] == CTRL_C {
                    break;
                }

                buf.push(b[0]);
            }
        };

        Ok(Value::binary(buf, call.head).into_pipeline_data())
    }
}
