use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnValue, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use std::{
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

const CTRL_C_CHECK_INTERVAL: Duration = Duration::from_millis(100);

pub struct Sleep;

#[derive(Deserialize)]
pub struct SleepArgs {
    pub duration: Tagged<u64>,
    pub rest: Vec<Tagged<u64>>,
}

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
        "Delay for a specified amount of time."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let ctrl_c = args.ctrl_c().clone();

        let (SleepArgs { duration, rest }, _) = args.process()?;

        let total_dur = Duration::from_nanos(duration.item)
            + rest
                .iter()
                .map(|val| Duration::from_nanos(val.item))
                .sum::<Duration>();

        //SleepHandler::new(total_dur, ctrl_c);
        // this is necessary because the following 2 commands gave different results:
        // `echo | sleep 1sec` - nothing
        // `sleep 1sec`        - table with 0 elements

        Ok(SleepIterator::new(total_dur, ctrl_c).to_output_stream())

        // if input.is_empty() {
        //     Ok(OutputStream::empty())
        // } else {
        //     Ok(input.into())
        // }
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
                description: "Send output after 1sec",
                example: "sleep 1sec; echo done",
                result: Some(vec![UntaggedValue::string("done").into()]),
            },
        ]
    }
}

struct SleepIterator {
    total_dur: Duration,
    ctrl_c: Arc<AtomicBool>,
}

impl SleepIterator {
    pub fn new(total_dur: Duration, ctrl_c: Arc<AtomicBool>) -> Self {
        Self { total_dur, ctrl_c }
    }
}

impl Iterator for SleepIterator {
    type Item = ReturnValue;

    fn next(&mut self) -> Option<Self::Item> {
        let start = Instant::now();
        loop {
            thread::sleep(CTRL_C_CHECK_INTERVAL);
            if start.elapsed() >= self.total_dur {
                break;
            }

            if self.ctrl_c.load(Ordering::SeqCst) {
                break;
            }
        }

        None
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
