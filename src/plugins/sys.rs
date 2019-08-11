#![feature(async_await)]

use futures::executor::block_on;
use futures::stream::StreamExt;
use heim::{disk, memory, net, sensors};
use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    Tag, Tagged, TaggedDictBuilder, Value,
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
    if let (Ok(num_cpu), Ok(cpu_speed)) = (
        heim::cpu::logical_count().await,
        heim::cpu::frequency().await,
    ) {
        let mut cpu_idx = TaggedDictBuilder::new(tag);
        cpu_idx.insert("cores", Primitive::Int(num_cpu as i64));

        let current_speed =
            (cpu_speed.current().get() as f64 / 1000000000.0 * 100.0).round() / 100.0;
        cpu_idx.insert("current ghz", Primitive::Float(current_speed.into()));

        if let Some(min_speed) = cpu_speed.min() {
            let min_speed = (min_speed.get() as f64 / 1000000000.0 * 100.0).round() / 100.0;
            cpu_idx.insert("min ghz", Primitive::Float(min_speed.into()));
        }

        if let Some(max_speed) = cpu_speed.max() {
            let max_speed = (max_speed.get() as f64 / 1000000000.0 * 100.0).round() / 100.0;
            cpu_idx.insert("max ghz", Primitive::Float(max_speed.into()));
        }
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

async fn disks(tag: Tag) -> Option<Value> {
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

    if output.len() > 0 {
        Some(Value::List(output))
    } else {
        None
    }
}

async fn battery(tag: Tag) -> Option<Value> {
    let mut output = vec![];

    if let Ok(manager) = battery::Manager::new() {
        if let Ok(batteries) = manager.batteries() {
            for battery in batteries {
                if let Ok(battery) = battery {
                    let mut dict = TaggedDictBuilder::new(tag);
                    if let Some(vendor) = battery.vendor() {
                        dict.insert("vendor", Value::string(vendor));
                    }
                    if let Some(model) = battery.model() {
                        dict.insert("model", Value::string(model));
                    }
                    if let Some(cycles) = battery.cycle_count() {
                        dict.insert("cycles", Value::int(cycles));
                    }
                    if let Some(time_to_full) = battery.time_to_full() {
                        dict.insert(
                            "mins to full",
                            Value::float(time_to_full.get::<battery::units::time::minute>() as f64),
                        );
                    }
                    if let Some(time_to_empty) = battery.time_to_empty() {
                        dict.insert(
                            "mins to empty",
                            Value::float(time_to_empty.get::<battery::units::time::minute>() as f64),
                        );
                    }
                    output.push(dict.into_tagged_value());
                }
            }
        }
    }

    if output.len() > 0 {
        Some(Value::List(output))
    } else {
        None
    }
}

async fn temp(tag: Tag) -> Option<Value> {
    let mut output = vec![];

    let mut sensors = sensors::temperatures();
    while let Some(sensor) = sensors.next().await {
        if let Ok(sensor) = sensor {
            let mut dict = TaggedDictBuilder::new(tag);
            dict.insert("unit", Value::string(sensor.unit()));
            if let Some(label) = sensor.label() {
                dict.insert("label", Value::string(label));
            }
            dict.insert("temp", Value::float(sensor.current().get()));
            if let Some(high) = sensor.high() {
                dict.insert("high", Value::float(high.get()));
            }
            if let Some(critical) = sensor.critical() {
                dict.insert("critical", Value::float(critical.get()));
            }

            output.push(dict.into_tagged_value());
        }
    }

    if output.len() > 0 {
        Some(Value::List(output))
    } else {
        None
    }
}

async fn net(tag: Tag) -> Option<Value> {
    let mut output = vec![];
    let mut io_counters = net::io_counters();
    while let Some(nic) = io_counters.next().await {
        if let Ok(nic) = nic {
            let mut network_idx = TaggedDictBuilder::new(tag);
            network_idx.insert("name", Value::string(nic.interface()));
            network_idx.insert("sent", Value::bytes(nic.bytes_sent().get()));
            network_idx.insert("recv", Value::bytes(nic.bytes_recv().get()));
            output.push(network_idx.into_tagged_value());
        }
    }
    if output.len() > 0 {
        Some(Value::List(output))
    } else {
        None
    }
}

async fn sysinfo(tag: Tag) -> Vec<Tagged<Value>> {
    let mut sysinfo = TaggedDictBuilder::new(tag);

    sysinfo.insert_tagged("host", host(tag).await);
    if let Some(cpu) = cpu(tag).await {
        sysinfo.insert_tagged("cpu", cpu);
    }
    if let Some(disks) = disks(tag).await {
        sysinfo.insert("disks", disks);
    }
    sysinfo.insert_tagged("mem", mem(tag).await);
    if let Some(temp) = temp(tag).await {
        sysinfo.insert("temp", temp);
    }
    if let Some(net) = net(tag).await {
        sysinfo.insert("net", net);
    }
    if let Some(battery) = battery(tag).await {
        sysinfo.insert("battery", battery);
    }

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
