use std::{sync::mpsc::RecvTimeoutError, time::Duration};

use nu_engine::command_prelude::*;
use nu_protocol::engine::FilterTag;

#[derive(Clone)]
pub struct JobRecv;

const CTRL_C_CHECK_INTERVAL: Duration = Duration::from_millis(100);

impl Command for JobRecv {
    fn name(&self) -> &str {
        "job recv"
    }

    fn description(&self) -> &str {
        "Read a message from the mailbox."
    }

    fn extra_description(&self) -> &str {
        r#"When messages are sent to the current process, they get stored in what is called the "mailbox".
This commands reads and returns a message from the mailbox, in a first-in-first-out fashion.
j
Messages may have numeric flags attached to them. This commands supports filtering out messages that do not satisfy a given tag, by using the `tag` flag.
If no tag is specified, this command will accept any message. 

If no message with the specified tag (if any) is available in the mailbox, this command will block the current thread until one arrives.
By default this command block indefinitely until a matching message arrives, but a timeout duration can be specified. 

Note: When using par-each, only one thread at a time can utilize this command.
In the case of two or more threads running this command, they will wait until other threads are done using it,
in no particular order, regardless of the specified timeout parameter.
"#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job recv")
            .category(Category::Experimental)
            .named("tag", SyntaxShape::Int, "A tag for the message", None)
            .named(
                "timeout",
                SyntaxShape::Duration,
                "The maximum time duration to wait for.",
                None,
            )
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["receive"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let tag_arg: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "tag")?;

        if let Some(tag) = tag_arg {
            if tag.item < 0 {
                return Err(ShellError::NeedsPositiveValue { span: tag.span });
            }
        }

        let tag = tag_arg.map(|it| it.item as FilterTag);

        let duration: Option<i64> = call.get_flag(engine_state, stack, "timeout")?;

        let timeout = duration.map(|it| Duration::from_nanos(it as u64));

        let mut mailbox = engine_state
            .current_job
            .mailbox
            .lock()
            .expect("failed to acquire lock");

        if let Some(timeout) = timeout {
            let value = mailbox
                .recv_timeout(tag, timeout)
                .map_err(|error| match error {
                    RecvTimeoutError::Timeout => ShellError::RecvTimeout { span: head },

                    // if the channel was disconnected, it means this job was removed from the job
                    // table, so it was killed/interrupted
                    RecvTimeoutError::Disconnected => ShellError::Interrupted { span: head },
                })?;

            Ok(value.into_pipeline_data())
        } else {
            loop {
                if engine_state.signals().interrupted() {
                    return Err(ShellError::Interrupted { span: head });
                }

                match mailbox.recv_timeout(tag, CTRL_C_CHECK_INTERVAL) {
                    Ok(value) => return Ok(value.into_pipeline_data()),
                    Err(RecvTimeoutError::Timeout) => {} // try again
                    Err(RecvTimeoutError::Disconnected) => {
                        return Err(ShellError::Interrupted { span: head })
                    }
                }
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "job recv",
            description: "Block the current thread while no message arrives",
            result: None,
        }]
    }
}
