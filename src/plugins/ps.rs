use futures::executor::block_on;
use futures::stream::{StreamExt, TryStreamExt};

use heim::process::{self as process, Process, ProcessResult};
use heim::units::{ratio, Ratio};
use std::usize;

use nu::{
    serve_plugin, CallInfo, Plugin, ReturnSuccess, ReturnValue, ShellError, Signature, Tag, Tagged,
    TaggedDictBuilder, Value,
};
use std::time::Duration;

struct Ps;
impl Ps {
    fn new() -> Ps {
        Ps
    }
}

async fn usage(process: Process) -> ProcessResult<(process::Process, Ratio)> {
    let usage_1 = process.cpu_usage().await?;
    futures_timer::Delay::new(Duration::from_millis(100)).await?;
    let usage_2 = process.cpu_usage().await?;

    Ok((process, usage_2 - usage_1))
}

async fn ps(tag: Tag) -> Vec<Tagged<Value>> {
    let processes = process::processes()
        .map_ok(|process| {
            // Note that there is no `.await` here,
            // as we want to pass the returned future
            // into the `.try_buffer_unordered`.
            usage(process)
        })
        .try_buffer_unordered(usize::MAX);
    pin_utils::pin_mut!(processes);

    let mut output = vec![];
    while let Some(res) = processes.next().await {
        if let Ok((process, usage)) = res {
            let mut dict = TaggedDictBuilder::new(&tag);
            dict.insert("pid", Value::int(process.pid()));
            if let Ok(name) = process.name().await {
                dict.insert("name", Value::string(name));
            }
            if let Ok(status) = process.status().await {
                dict.insert("status", Value::string(format!("{:?}", status)));
            }
            dict.insert("cpu", Value::number(usage.get::<ratio::percent>()));
            output.push(dict.into_tagged_value());
        }
    }

    output
}

impl Plugin for Ps {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("ps")
            .desc("View information about system processes.")
            .filter())
    }

    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(block_on(ps(callinfo.name_tag))
            .into_iter()
            .map(ReturnSuccess::value)
            .collect())
    }

    fn filter(&mut self, _: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}

fn main() {
    serve_plugin(&mut Ps::new());
}
