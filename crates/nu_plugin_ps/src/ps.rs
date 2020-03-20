use futures::{StreamExt, TryStreamExt};
use heim::process::{self as process, Process, ProcessResult};
use heim::units::{information, ratio, Ratio};
use std::usize;

use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;

use std::time::Duration;

#[derive(Default)]
pub struct Ps;

impl Ps {
    pub fn new() -> Ps {
        Ps
    }
}

async fn usage(process: Process) -> ProcessResult<(process::Process, Ratio, process::Memory)> {
    let usage_1 = process.cpu_usage().await?;
    futures_timer::Delay::new(Duration::from_millis(100)).await;
    let usage_2 = process.cpu_usage().await?;

    let memory = process.memory().await?;

    Ok((process, usage_2 - usage_1, memory))
}

pub async fn ps(tag: Tag, full: bool) -> Vec<Value> {
    let mut processes = process::processes()
        .map_ok(|process| {
            // Note that there is no `.await` here,
            // as we want to pass the returned future
            // into the `.try_buffer_unordered`.
            usage(process)
        })
        .try_buffer_unordered(usize::MAX);

    let mut output = vec![];
    while let Some(res) = processes.next().await {
        if let Ok((process, usage, memory)) = res {
            let mut dict = TaggedDictBuilder::new(&tag);
            dict.insert_untagged("pid", UntaggedValue::int(process.pid()));
            if let Ok(name) = process.name().await {
                dict.insert_untagged("name", UntaggedValue::string(name));
            }
            if let Ok(status) = process.status().await {
                dict.insert_untagged("status", UntaggedValue::string(format!("{:?}", status)));
            }
            dict.insert_untagged("cpu", UntaggedValue::decimal(usage.get::<ratio::percent>()));
            dict.insert_untagged(
                "mem",
                UntaggedValue::bytes(memory.rss().get::<information::byte>()),
            );
            dict.insert_untagged(
                "virtual",
                UntaggedValue::bytes(memory.vms().get::<information::byte>()),
            );
            if full {
                if let Ok(parent_pid) = process.parent_pid().await {
                    dict.insert_untagged("parent", UntaggedValue::int(parent_pid))
                }

                if let Ok(exe) = process.exe().await {
                    dict.insert_untagged("exe", UntaggedValue::string(exe.to_string_lossy()))
                }

                if let Ok(command) = process.command().await {
                    dict.insert_untagged(
                        "command",
                        UntaggedValue::string(command.to_os_string().to_string_lossy()),
                    );
                }
            }
            output.push(dict.into_value());
        }
    }

    output
}
