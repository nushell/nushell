use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

use std::{thread, time};

pub struct Sleep;

#[derive(Deserialize)]
pub struct SleepArgs {
    pub dur: Tagged<u64>,
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
        "delay for a specified amount of time"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        sleep(args, registry).await
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
        ]
    }
}

async fn sleep(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let (SleepArgs { dur, rest }, ..) = args.process(&registry).await?;

    let total_dur = dur.item + rest.iter().map(|val| val.item).sum::<u64>();
    let total_dur = time::Duration::from_nanos(total_dur);
    thread::sleep(total_dur);

    Ok(OutputStream::empty())
}

#[cfg(test)]
mod tests {
    use super::Sleep;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        let start = Instant::now();
        test_examples(Sleep {});
        let elapsed = start.elapsed();
        println!("{:?}", elapsed);
        assert!(elapsed >= std::time::Duration::from_secs(4));
    }
}
