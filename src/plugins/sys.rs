#![feature(async_await)]

use futures::executor::block_on;
use futures::stream::StreamExt;
use heim::{disk, memory};
use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    Tag, Tagged, TaggedDictBuilder, Value, OF64,
};
use std::ffi::OsStr;

struct Sys;
impl Sys {
    fn new() -> Sys {
        Sys
    }
}

//TODO: add more error checking

async fn cpu(tag: Tag) -> Option<Tagged<Value>> {
    if let (Ok(num_cpu), Ok(cpu_speed)) = (sys_info::cpu_num(), sys_info::cpu_speed()) {
        let mut cpu_idx = TaggedDictBuilder::new(tag);
        cpu_idx.insert("cores", Primitive::Int(num_cpu as i64));
        cpu_idx.insert("speed", Primitive::Int(cpu_speed as i64));
        Some(cpu_idx.into_tagged_value())
    } else {
        None
    }
}

async fn mem(tag: Tag) -> Tagged<Value> {
    let mut dict = TaggedDictBuilder::new(tag);

    if let Ok(memory) = memory::memory().await {
        dict.insert("total", Value::bytes(memory.total().get()));
        dict.insert("free", Value::bytes(memory.free().get()));
    }
    if let Ok(swap) = memory::swap().await {
        dict.insert("swap total", Value::bytes(swap.total().get()));
        dict.insert("swap free", Value::bytes(swap.free().get()));
    }

    dict.into_tagged_value()
}

async fn host(tag: Tag) -> Tagged<Value> {
    let mut dict = TaggedDictBuilder::new(tag);

    // OS
    if let Ok(platform) = heim::host::platform().await {
        dict.insert("name", Value::string(platform.system()));
        dict.insert("release", Value::string(platform.release()));
        dict.insert("hostname", Value::string(platform.hostname()));
        dict.insert("arch", Value::string(platform.architecture().as_str()));
    }

    // Uptime
    if let Ok(uptime) = heim::host::uptime().await {
        let mut uptime_dict = TaggedDictBuilder::new(tag);

        let uptime = uptime.get().round() as i64;
        let days = uptime / (60 * 60 * 24);
        let hours = (uptime - days * 60 * 60 * 24) / (60 * 60);
        let minutes = (uptime - days * 60 * 60 * 24 - hours * 60 * 60) / 60;
        let seconds = uptime % 60;

        uptime_dict.insert("days", Value::int(days));
        uptime_dict.insert("hours", Value::int(hours));
        uptime_dict.insert("mins", Value::int(minutes));
        uptime_dict.insert("secs", Value::int(seconds));

        dict.insert_tagged("uptime", uptime_dict.into_tagged_value());
    }

    // Users
    let mut users = heim::host::users();
    let mut user_vec = vec![];
    while let Some(user) = users.next().await {
        if let Ok(user) = user {
            user_vec.push(Tagged::from_item(Value::string(user.username()), tag));
        }
    }
    let user_list = Value::List(user_vec);
    dict.insert("users", user_list);

    dict.into_tagged_value()
}

async fn disks(tag: Tag) -> Value {
    let mut output = vec![];
    let mut partitions = disk::partitions_physical();
    while let Some(part) = partitions.next().await {
        if let Ok(part) = part {
            let mut dict = TaggedDictBuilder::new(tag);
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
            if let Ok(usage) = disk::usage(part.mount_point().to_path_buf()).await {
                dict.insert("total", Value::bytes(usage.total().get()));
                dict.insert("used", Value::bytes(usage.used().get()));
                dict.insert("free", Value::bytes(usage.free().get()));
            }
            output.push(dict.into_tagged_value());
        }
    }

    Value::List(output)
}

async fn temp(tag: Tag) -> Value {
    use sysinfo::{ComponentExt, RefreshKind, SystemExt};
    let system = sysinfo::System::new_with_specifics(RefreshKind::new().with_system());
    let components_list = system.get_components_list();
    if components_list.len() > 0 {
        let mut v: Vec<Tagged<Value>> = vec![];
        for component in components_list {
            let mut component_idx = TaggedDictBuilder::new(tag);
            component_idx.insert("name", Primitive::String(component.get_label().to_string()));
            component_idx.insert(
                "temp",
                Primitive::Float(OF64::from(component.get_temperature() as f64)),
            );
            component_idx.insert(
                "max",
                Primitive::Float(OF64::from(component.get_max() as f64)),
            );
            if let Some(critical) = component.get_critical() {
                component_idx.insert("critical", Primitive::Float(OF64::from(critical as f64)));
            }
            v.push(component_idx.into());
        }
        Value::List(v)
    } else {
        Value::List(vec![])
    }
}

async fn net(tag: Tag) -> Tagged<Value> {
    use sysinfo::{NetworkExt, RefreshKind, SystemExt};
    let system = sysinfo::System::new_with_specifics(RefreshKind::new().with_network());

    let network = system.get_network();
    let incoming = network.get_income();
    let outgoing = network.get_outcome();

    let mut network_idx = TaggedDictBuilder::new(tag);
    network_idx.insert("incoming", Value::bytes(incoming));
    network_idx.insert("outgoing", Value::bytes(outgoing));
    network_idx.into_tagged_value()
}

async fn sysinfo(tag: Tag) -> Vec<Tagged<Value>> {
    let mut sysinfo = TaggedDictBuilder::new(tag);

    sysinfo.insert_tagged("host", host(tag).await);
    if let Some(cpu) = cpu(tag).await {
        sysinfo.insert_tagged("cpu", cpu);
    }
    sysinfo.insert("disks", disks(tag).await);
    sysinfo.insert_tagged("mem", mem(tag).await);
    sysinfo.insert("temp", temp(tag).await);
    sysinfo.insert_tagged("net", net(tag).await);

    vec![sysinfo.into_tagged_value()]
}

impl Plugin for Sys {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature {
            name: "sys".to_string(),
            positional: vec![],
            is_filter: true,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }
    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(block_on(sysinfo(Tag::unknown_origin(callinfo.name_span)))
            .into_iter()
            .map(|x| ReturnSuccess::value(x))
            .collect())
    }

    fn filter(&mut self, _: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}

fn main() {
    serve_plugin(&mut Sys::new());
}
