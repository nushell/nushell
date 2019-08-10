#![feature(async_await)]

use futures::executor::block_on;
use futures::stream::StreamExt;
use heim::process;
use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, Plugin, ReturnSuccess, ReturnValue, ShellError, Signature, Tag, Tagged,
    TaggedDictBuilder, Value,
};

struct Ps;
impl Ps {
    fn new() -> Ps {
        Ps
    }
}

async fn ps(tag: Tag) -> Vec<Tagged<Value>> {
    let mut output = vec![];

    let mut process = process::processes();
    while let Some(process) = process.next().await {
        if let Ok(process) = process {
            let mut dict = TaggedDictBuilder::new(tag);
            dict.insert("pid", Value::int(process.pid() as i64));
            if let Ok(parent) = process.parent_pid().await {
                dict.insert("parent", Value::int(parent as i64));
            }
            if let Ok(status) = process.status().await {
                dict.insert("status", Value::string(format!("{:?}", status)));
            }
            if let Ok(name) = process.name().await {
                dict.insert("name", Value::string(name));
            }
            if let Ok(exe) = process.exe().await {
                dict.insert("exe", Value::string(exe.to_string_lossy().to_string()));
            }

            output.push(dict.into_tagged_value());
        }
    }

    output
}

impl Plugin for Ps {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature {
            name: "ps".to_string(),
            positional: vec![],
            is_filter: true,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }
    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(block_on(ps(Tag::unknown_origin(callinfo.name_span)))
            .into_iter()
            .map(|x| ReturnSuccess::value(x))
            .collect())
    }

    fn filter(&mut self, _: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}

fn main() {
    serve_plugin(&mut Ps::new());
}
