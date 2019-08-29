use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::TaggedDictBuilder;
use crate::prelude::*;
use std::time::Duration;
use std::usize;

use futures::stream::{StreamExt, TryStreamExt};
use heim::process::{self as process, Process, ProcessResult};
use heim::units::{ratio, Ratio};

pub struct PS;

impl WholeStreamCommand for PS {
    fn name(&self) -> &str {
        "ps"
    }

    fn signature(&self) -> Signature {
        Signature::build("ps")
    }

    fn usage(&self) -> &str {
        "View current processes."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        ps(args, registry)
    }
}

async fn usage(process: Process) -> ProcessResult<(process::Process, Ratio)> {
    let usage_1 = process.cpu_usage().await?;
    futures_timer::Delay::new(Duration::from_millis(100)).await?;
    let usage_2 = process.cpu_usage().await?;

    Ok((process, usage_2 - usage_1))
}

fn ps(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.name_span();

    let stream = async_stream_block! {
        let processes = process::processes()
            .map_ok(|process| {
                // Note that there is no `.await` here,
                // as we want to pass the returned future
                // into the `.try_buffer_unordered`.
                usage(process)
            })
            .try_buffer_unordered(usize::MAX);
        pin_utils::pin_mut!(processes);

        while let Some(res) = processes.next().await {
            if let Ok((process, usage)) = res {
                let mut dict = TaggedDictBuilder::new(Tag::unknown_origin(span));
                dict.insert("pid", Value::int(process.pid()));
                if let Ok(name) = process.name().await {
                    dict.insert("name", Value::string(name));
                }
                if let Ok(status) = process.status().await {
                    dict.insert("status", Value::string(format!("{:?}", status)));
                }
                dict.insert("cpu", Value::number(usage.get::<ratio::percent>()));
                yield ReturnSuccess::value(dict.into_tagged_value());
            }
        }
    };

    Ok(stream.to_output_stream())
}
