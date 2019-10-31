use std::ffi::OsStr;

use futures::executor::block_on;
use futures::stream::StreamExt;
use heim::units::{frequency, information, thermodynamic_temperature, time};
use heim::{disk, host, memory, net, sensors};
use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    Tag, Tagged, TaggedDictBuilder, Value,
};

struct Sys;
impl Sys {
    fn new() -> Sys {
        Sys
    }
}

async fn cpu(tag: Tag) -> Option<Tagged<Value>> {
    match futures::future::try_join(heim::cpu::logical_count(), heim::cpu::frequency()).await {
        Ok((num_cpu, cpu_speed)) => {
            let mut cpu_idx = TaggedDictBuilder::with_capacity(tag, 4);
            cpu_idx.insert("cores", Primitive::number(num_cpu));

            let current_speed =
                (cpu_speed.current().get::<frequency::hertz>() as f64 / 1_000_000_000.0 * 100.0)
                    .round()
                    / 100.0;
            cpu_idx.insert("current ghz", Primitive::number(current_speed));

            if let Some(min_speed) = cpu_speed.min() {
                let min_speed =
                    (min_speed.get::<frequency::hertz>() as f64 / 1_000_000_000.0 * 100.0).round()
                        / 100.0;
                cpu_idx.insert("min ghz", Primitive::number(min_speed));
            }

            if let Some(max_speed) = cpu_speed.max() {
                let max_speed =
                    (max_speed.get::<frequency::hertz>() as f64 / 1_000_000_000.0 * 100.0).round()
                        / 100.0;
                cpu_idx.insert("max ghz", Primitive::number(max_speed));
            }

            Some(cpu_idx.into_tagged_value())
        }
        Err(_) => None,
    }
}

async fn mem(tag: Tag) -> Tagged<Value> {
    let mut dict = TaggedDictBuilder::with_capacity(tag, 4);

    let (memory_result, swap_result) =
        futures::future::join(memory::memory(), memory::swap()).await;

    if let Ok(memory) = memory_result {
        dict.insert(
            "total",
            Value::bytes(memory.total().get::<information::byte>()),
        );
        dict.insert(
            "free",
            Value::bytes(memory.free().get::<information::byte>()),
        );
    }

    if let Ok(swap) = swap_result {
        dict.insert(
            "swap total",
            Value::bytes(swap.total().get::<information::byte>()),
        );
        dict.insert(
            "swap free",
            Value::bytes(swap.free().get::<information::byte>()),
        );
    }

    dict.into_tagged_value()
}

async fn host(tag: Tag) -> Tagged<Value> {
    let mut dict = TaggedDictBuilder::with_capacity(&tag, 6);

    let (platform_result, uptime_result) =
        futures::future::join(host::platform(), host::uptime()).await;

    // OS
    if let Ok(platform) = platform_result {
        dict.insert("name", Value::string(platform.system()));
        dict.insert("release", Value::string(platform.release()));
        dict.insert("hostname", Value::string(platform.hostname()));
        dict.insert("arch", Value::string(platform.architecture().as_str()));
    }

    // Uptime
    if let Ok(uptime) = uptime_result {
        let mut uptime_dict = TaggedDictBuilder::with_capacity(&tag, 4);

        let uptime = uptime.get::<time::second>().round() as i64;
        let days = uptime / (60 * 60 * 24);
        let hours = (uptime - days * 60 * 60 * 24) / (60 * 60);
        let minutes = (uptime - days * 60 * 60 * 24 - hours * 60 * 60) / 60;
        let seconds = uptime % 60;

        uptime_dict.insert("days", Value::int(days));
        uptime_dict.insert("hours", Value::int(hours));
        uptime_dict.insert("mins", Value::int(minutes));
        uptime_dict.insert("secs", Value::int(seconds));

        dict.insert_tagged("uptime", uptime_dict);
    }

    // Users
    let mut users = host::users();
    let mut user_vec = vec![];
    while let Some(user) = users.next().await {
        if let Ok(user) = user {
            user_vec.push(Tagged {
                item: Value::string(user.username()),
                tag: tag.clone(),
            });
        }
    }
    let user_list = Value::Table(user_vec);
    dict.insert("users", user_list);

    dict.into_tagged_value()
}

async fn disks(tag: Tag) -> Option<Value> {
    let mut output = vec![];
    let mut partitions = disk::partitions_physical();
    while let Some(part) = partitions.next().await {
        if let Ok(part) = part {
            let mut dict = TaggedDictBuilder::with_capacity(&tag, 6);
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
                dict.insert(
                    "total",
                    Value::bytes(usage.total().get::<information::byte>()),
                );
                dict.insert(
                    "used",
                    Value::bytes(usage.used().get::<information::byte>()),
                );
                dict.insert(
                    "free",
                    Value::bytes(usage.free().get::<information::byte>()),
                );
            }

            output.push(dict.into_tagged_value());
        }
    }

    if !output.is_empty() {
        Some(Value::Table(output))
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
                    let mut dict = TaggedDictBuilder::new(&tag);
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
                            Value::number(time_to_full.get::<battery::units::time::minute>()),
                        );
                    }
                    if let Some(time_to_empty) = battery.time_to_empty() {
                        dict.insert(
                            "mins to empty",
                            Value::number(time_to_empty.get::<battery::units::time::minute>()),
                        );
                    }
                    output.push(dict.into_tagged_value());
                }
            }
        }
    }

    if !output.is_empty() {
        Some(Value::Table(output))
    } else {
        None
    }
}

async fn temp(tag: Tag) -> Option<Value> {
    let mut output = vec![];

    let mut sensors = sensors::temperatures();
    while let Some(sensor) = sensors.next().await {
        if let Ok(sensor) = sensor {
            let mut dict = TaggedDictBuilder::new(&tag);
            dict.insert("unit", Value::string(sensor.unit()));
            if let Some(label) = sensor.label() {
                dict.insert("label", Value::string(label));
            }
            dict.insert(
                "temp",
                Value::number(
                    sensor
                        .current()
                        .get::<thermodynamic_temperature::degree_celsius>(),
                ),
            );
            if let Some(high) = sensor.high() {
                dict.insert(
                    "high",
                    Value::number(high.get::<thermodynamic_temperature::degree_celsius>()),
                );
            }
            if let Some(critical) = sensor.critical() {
                dict.insert(
                    "critical",
                    Value::number(critical.get::<thermodynamic_temperature::degree_celsius>()),
                );
            }

            output.push(dict.into_tagged_value());
        }
    }

    if !output.is_empty() {
        Some(Value::Table(output))
    } else {
        None
    }
}

async fn net(tag: Tag) -> Option<Value> {
    let mut output = vec![];
    let mut io_counters = net::io_counters();
    while let Some(nic) = io_counters.next().await {
        if let Ok(nic) = nic {
            let mut network_idx = TaggedDictBuilder::with_capacity(&tag, 3);
            network_idx.insert("name", Value::string(nic.interface()));
            network_idx.insert(
                "sent",
                Value::bytes(nic.bytes_sent().get::<information::byte>()),
            );
            network_idx.insert(
                "recv",
                Value::bytes(nic.bytes_recv().get::<information::byte>()),
            );
            output.push(network_idx.into_tagged_value());
        }
    }
    if !output.is_empty() {
        Some(Value::Table(output))
    } else {
        None
    }
}

async fn sysinfo(tag: Tag) -> Vec<Tagged<Value>> {
    let mut sysinfo = TaggedDictBuilder::with_capacity(&tag, 7);

    let (host, cpu, disks, memory, temp) = futures::future::join5(
        host(tag.clone()),
        cpu(tag.clone()),
        disks(tag.clone()),
        mem(tag.clone()),
        temp(tag.clone()),
    )
    .await;
    let (net, battery) = futures::future::join(net(tag.clone()), battery(tag.clone())).await;

    sysinfo.insert_tagged("host", host);
    if let Some(cpu) = cpu {
        sysinfo.insert_tagged("cpu", cpu);
    }
    if let Some(disks) = disks {
        sysinfo.insert("disks", disks);
    }
    sysinfo.insert_tagged("mem", memory);
    if let Some(temp) = temp {
        sysinfo.insert("temp", temp);
    }
    if let Some(net) = net {
        sysinfo.insert("net", net);
    }
    if let Some(battery) = battery {
        sysinfo.insert("battery", battery);
    }

    vec![sysinfo.into_tagged_value()]
}

impl Plugin for Sys {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("sys")
            .desc("View information about the current system.")
            .filter())
    }

    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(block_on(sysinfo(callinfo.name_tag))
            .into_iter()
            .map(ReturnSuccess::value)
            .collect())
    }

    fn filter(&mut self, _: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}

fn main() {
    serve_plugin(&mut Sys::new());
}
