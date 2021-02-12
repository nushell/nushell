use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use parking_lot::Mutex;
use std::{
    future::Future,
    pin::Pin,
    sync::atomic::Ordering,
    task::{Poll, Waker},
    thread,
    time::Duration,
};

const CTRL_C_CHECK_INTERVAL: Duration = Duration::from_millis(100);

pub struct Sleep;

#[derive(Deserialize)]
pub struct SleepArgs {
    pub duration: Tagged<u64>,
    pub rest: Vec<Tagged<u64>>,
}

#[async_trait]
impl WholeStreamCommand for Sleep {
    fn name(&self) -> &str {
        "sleep"
    }

    fn signature(&self) -> Signature {
        Signature::build("sleep")
            .required("duration", SyntaxShape::Unit, "time to sleep")
            .rest(SyntaxShape::Unit, "additional time")
    }

    fn usage(&self) -> &str {
        "Delay for a specified amount of time"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let ctrl_c = args.ctrl_c().clone();

        let (SleepArgs { duration, rest }, input) = args.process().await?;

        let total_dur = Duration::from_nanos(duration.item)
            + rest
                .iter()
                .map(|val| Duration::from_nanos(val.item))
                .sum::<Duration>();

        SleepFuture::new(total_dur, ctrl_c).await;
        // this is necessary because the following 2 commands gave different results:
        // `echo | sleep 1sec` - nothing
        // `sleep 1sec`        - table with 0 elements
        if input.is_empty() {
            Ok(OutputStream::empty())
        } else {
            Ok(input.into())
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sleep for 1sec",
                example: "sleep 1sec",
                result: None,
            },
            Example {
                description: "Sleep for 3sec",
                example: "sleep 1sec 1sec 1sec",
                result: None,
            },
            Example {
                description: "Delay the output of another command by 1sec",
                example: "echo [55 120] | sleep 1sec",
                result: Some(vec![
                    UntaggedValue::int(55).into(),
                    UntaggedValue::int(120).into(),
                ]),
            },
        ]
    }
}

struct SleepFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

impl SleepFuture {
    /// Create a new `SleepFuture` which will complete after the provided
    /// timeout and check for Ctrl+C periodically.
    pub fn new(duration: Duration, ctrl_c: Arc<AtomicBool>) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            done: false,
            waker: None,
        }));

        // Spawn the main sleep thread
        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            thread::sleep(duration);
            let mut shared_state = thread_shared_state.lock();
            // Signal that the timer has completed and wake up the last
            // task on which the future was polled, if one exists.
            if !shared_state.done {
                shared_state.done = true;
                if let Some(waker) = shared_state.waker.take() {
                    waker.wake()
                }
            }
        });

        // Spawn the Ctrl+C-watching polling thread
        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            loop {
                {
                    let mut shared_state = thread_shared_state.lock();
                    // exit if the main thread is done
                    if shared_state.done {
                        return;
                    }
                    // finish the future prematurely if Ctrl+C has been pressed
                    if ctrl_c.load(Ordering::SeqCst) {
                        shared_state.done = true;
                        if let Some(waker) = shared_state.waker.take() {
                            waker.wake()
                        }
                        return;
                    }
                }
                // sleep for a short time
                thread::sleep(CTRL_C_CHECK_INTERVAL);
            }
        });

        SleepFuture { shared_state }
    }
}

struct SharedState {
    done: bool,
    waker: Option<Waker>,
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        // Look at the shared state to see if the timer has already completed.
        let mut shared_state = self.shared_state.lock();
        if shared_state.done {
            Poll::Ready(())
        } else {
            // Set the waker if necessary
            if shared_state
                .waker
                .as_ref()
                .map(|waker| !waker.will_wake(&cx.waker()))
                .unwrap_or(true)
            {
                shared_state.waker = Some(cx.waker().clone());
            }
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sleep;
    use nu_errors::ShellError;
    use std::time::Instant;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        let start = Instant::now();
        let results = test_examples(Sleep {});
        let elapsed = start.elapsed();
        println!("{:?}", elapsed);
        // only examples with actual output are run
        assert!(elapsed >= std::time::Duration::from_secs(1));
        assert!(elapsed < std::time::Duration::from_secs(2));

        results
    }
}
