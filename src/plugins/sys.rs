#![feature(async_await)]

use futures::executor::block_on;
use futures::stream::StreamExt;
use heim::{disk, memory};
use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, CommandConfig, Plugin, ReturnSuccess, ReturnValue, ShellError, Span,
    Spanned, SpannedDictBuilder, Value,
};
use std::ffi::OsStr;

struct Sys;
impl Sys {
    fn new() -> Sys {
        Sys
    }
}

//TODO: add more error checking

async fn mem(span: Span) -> Spanned<Value> {
    let memory = memory::memory().await.unwrap();
    //let swap = memory::swap().await.unwrap();

    let mut dict = SpannedDictBuilder::new(span);

    dict.insert("total", Value::bytes(memory.total().get()));
    dict.insert("free", Value::bytes(memory.free().get()));

    dict.into_spanned_value()
}

async fn swap(span: Span) -> Spanned<Value> {
    let swap = memory::swap().await.unwrap();

    let mut dict = SpannedDictBuilder::new(span);

    dict.insert("total", Value::bytes(swap.total().get()));
    dict.insert("free", Value::bytes(swap.free().get()));

    dict.into_spanned_value()
}

async fn disks(span: Span) -> Value {
    let mut output = vec![];
    let mut partitions = disk::partitions_physical();
    while let Some(part) = partitions.next().await {
        let part = part.unwrap();
        let usage = disk::usage(part.mount_point().to_path_buf()).await.unwrap();

        let mut dict = SpannedDictBuilder::new(span);

        dict.insert(
            "device",
            Value::string(
                part.device()
                    .unwrap_or_else(|| OsStr::new("N/A"))
                    .to_string_lossy(),
            ),
        );

        dict.insert("type", Value::string(part.file_system().as_str()));
        dict.insert("mount", Value::string(part.mount_point().to_string_lossy()));
        dict.insert("total", Value::bytes(usage.total().get()));
        dict.insert("used", Value::bytes(usage.used().get()));
        dict.insert("free", Value::bytes(usage.free().get()));

        output.push(dict.into_spanned_value());
    }

    Value::List(output)
}

async fn sysinfo(span: Span) -> Vec<Spanned<Value>> {
    let mut sysinfo = SpannedDictBuilder::new(span);

    // Disks
    sysinfo.insert("disks", disks(span).await);
    sysinfo.insert_spanned("mem", mem(span).await);
    sysinfo.insert_spanned("swap", swap(span).await);

    vec![sysinfo.into_spanned_value()]
}

impl Plugin for Sys {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        Ok(CommandConfig {
            name: "sys".to_string(),
            positional: vec![],
            is_filter: true,
            is_sink: false,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }
    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(block_on(sysinfo(callinfo.name_span.unwrap()))
            .into_iter()
            .map(|x| ReturnSuccess::value(x))
            .collect())
    }

    fn filter(&mut self, _: Spanned<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}

fn main() {
    serve_plugin(&mut Sys::new());
}
