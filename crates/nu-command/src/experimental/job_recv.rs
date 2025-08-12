use std::{
    sync::mpsc::{RecvTimeoutError, TryRecvError},
    time::{Duration, Instant},
};

use nu_engine::command_prelude::*;

use nu_protocol::{
    Signals,
    engine::{FilterTag, Mailbox},
};

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

Messages may have numeric flags attached to them. This commands supports filtering out messages that do not satisfy a given tag, by using the `tag` flag.
If no tag is specified, this command will accept any message.

If no message with the specified tag (if any) is available in the mailbox, this command will block the current thread until one arrives.
By default this command block indefinitely until a matching message arrives, but a timeout duration can be specified.
If a timeout duration of zero is specified, it will succeed only if there already is a message in the mailbox.

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

        let timeout: Option<Duration> = call.get_flag(engine_state, stack, "timeout")?;

        let mut mailbox = engine_state
            .current_job
            .mailbox
            .lock()
            .expect("failed to acquire lock");

        if let Some(timeout) = timeout {
            if timeout == Duration::ZERO {
                recv_instantly(&mut mailbox, tag, head)
            } else {
                recv_with_time_limit(&mut mailbox, tag, engine_state.signals(), head, timeout)
            }
        } else {
            recv_without_time_limit(&mut mailbox, tag, engine_state.signals(), head)
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "job recv",
                description: "Block the current thread while no message arrives",
                result: None,
            },
            Example {
                example: "job recv --timeout 10sec",
                description: "Receive a message, wait for at most 10 seconds.",
                result: None,
            },
            Example {
                example: "job recv --timeout 0sec",
                description: "Get a message or fail if no message is available immediately",
                result: None,
            },
            Example {
                example: "job spawn { sleep 1sec; 'hi' | job send 0 }; job recv",
                description: "Receive a message from a newly-spawned job",
                result: None,
            },
        ]
    }
}

fn recv_without_time_limit(
    mailbox: &mut Mailbox,
    tag: Option<FilterTag>,
    signals: &Signals,
    span: Span,
) -> Result<PipelineData, ShellError> {
    loop {
        if signals.interrupted() {
            return Err(ShellError::Interrupted { span });
        }
        match mailbox.recv_timeout(tag, CTRL_C_CHECK_INTERVAL) {
            Ok(value) => return Ok(value),
            Err(RecvTimeoutError::Timeout) => {} // try again
            Err(RecvTimeoutError::Disconnected) => return Err(ShellError::Interrupted { span }),
        }
    }
}

fn recv_instantly(
    mailbox: &mut Mailbox,
    tag: Option<FilterTag>,
    span: Span,
) -> Result<PipelineData, ShellError> {
    match mailbox.try_recv(tag) {
        Ok(value) => Ok(value),
        Err(TryRecvError::Empty) => Err(JobError::RecvTimeout { span }.into()),
        Err(TryRecvError::Disconnected) => Err(ShellError::Interrupted { span }),
    }
}

fn recv_with_time_limit(
    mailbox: &mut Mailbox,
    tag: Option<FilterTag>,
    signals: &Signals,
    span: Span,
    timeout: Duration,
) -> Result<PipelineData, ShellError> {
    let deadline = Instant::now() + timeout;

    loop {
        if signals.interrupted() {
            return Err(ShellError::Interrupted { span });
        }

        let time_until_deadline = deadline.saturating_duration_since(Instant::now());

        let time_to_sleep = time_until_deadline.min(CTRL_C_CHECK_INTERVAL);

        match mailbox.recv_timeout(tag, time_to_sleep) {
            Ok(value) => return Ok(value),
            Err(RecvTimeoutError::Timeout) => {} // try again
            Err(RecvTimeoutError::Disconnected) => return Err(ShellError::Interrupted { span }),
        }

        if time_until_deadline.is_zero() {
            return Err(JobError::RecvTimeout { span }.into());
        }
    }
}
