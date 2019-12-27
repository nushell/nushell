use std::ffi::OsStr;

use futures::executor::block_on;
//use futures::stream::StreamExt;
use futures_util::StreamExt;
use heim::units::{frequency, information, thermodynamic_temperature, time};
use heim::{disk, host, memory, net, sensors};
use nu_errors::ShellError;
use nu_plugin::{serve_plugin, Plugin};
use nu_protocol::{
    CallInfo, ReturnSuccess, ReturnValue, Signature, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tag;

struct Sys;
impl Sys {
    fn new() -> Sys {
        Sys
    }
}

async fn cpu(tag: Tag) -> Option<Value> {
    match futures::future::try_join(heim::cpu::logical_count(), heim::cpu::frequency()).await {
        Ok((num_cpu, cpu_speed)) => {
            let mut cpu_idx = TaggedDictBuilder::with_capacity(tag, 4);
            cpu_idx.insert_untagged("cores", UntaggedValue::int(num_cpu));

            let current_speed =
                (cpu_speed.current().get::<frequency::hertz>() as f64 / 1_000_000_000.0 * 100.0)
                    .round()
                    / 100.0;
            cpu_idx.insert_untagged("current ghz", UntaggedValue::decimal(current_speed));

            if let Some(min_speed) = cpu_speed.min() {
                let min_speed =
                    (min_speed.get::<frequency::hertz>() as f64 / 1_000_000_000.0 * 100.0).round()
                        / 100.0;
                cpu_idx.insert_untagged("min ghz", UntaggedValue::decimal(min_speed));
            }

            if let Some(max_speed) = cpu_speed.max() {
                let max_speed =
                    (max_speed.get::<frequency::hertz>() as f64 / 1_000_000_000.0 * 100.0).round()
                        / 100.0;
                cpu_idx.insert_untagged("max ghz", UntaggedValue::decimal(max_speed));
            }

            Some(cpu_idx.into_value())
        }
        Err(_) => None,
    }
}

async fn mem(tag: Tag) -> Value {
    let mut dict = TaggedDictBuilder::with_capacity(tag, 4);

    let (memory_result, swap_result) =
        futures::future::join(memory::memory(), memory::swap()).await;

    if let Ok(memory) = memory_result {
        dict.insert_untagged(
            "total",
            UntaggedValue::bytes(memory.total().get::<information::byte>()),
        );
        dict.insert_untagged(
            "free",
            UntaggedValue::bytes(memory.free().get::<information::byte>()),
        );
    }

    if let Ok(swap) = swap_result {
        dict.insert_untagged(
            "swap total",
            UntaggedValue::bytes(swap.total().get::<information::byte>()),
        );
        dict.insert_untagged(
            "swap free",
            UntaggedValue::bytes(swap.free().get::<information::byte>()),
        );
    }

    dict.into_value()
}

async fn host(tag: Tag) -> Value {
    let mut dict = TaggedDictBuilder::with_capacity(&tag, 6);

    let (platform_result, uptime_result) =
        futures::future::join(host::platform(), host::uptime()).await;

    // OS
    if let Ok(platform) = platform_result {
        dict.insert_untagged("name", UntaggedValue::string(platform.system()));
        dict.insert_untagged("release", UntaggedValue::string(platform.release()));
        dict.insert_untagged("hostname", UntaggedValue::string(platform.hostname()));
        dict.insert_untagged(
            "arch",
            UntaggedValue::string(platform.architecture().as_str()),
        );
    }

    // Uptime
    if let Ok(uptime) = uptime_result {
        let mut uptime_dict = TaggedDictBuilder::with_capacity(&tag, 4);

        let uptime = uptime.get::<time::second>().round() as i64;
        let days = uptime / (60 * 60 * 24);
        let hours = (uptime - days * 60 * 60 * 24) / (60 * 60);
        let minutes = (uptime - days * 60 * 60 * 24 - hours * 60 * 60) / 60;
        let seconds = uptime % 60;

        uptime_dict.insert_untagged("days", UntaggedValue::int(days));
        uptime_dict.insert_untagged("hours", UntaggedValue::int(hours));
        uptime_dict.insert_untagged("mins", UntaggedValue::int(minutes));
        uptime_dict.insert_untagged("secs", UntaggedValue::int(seconds));

        dict.insert_value("uptime", uptime_dict);
    }

    // Users
    let mut users = host::users();
    let mut user_vec = vec![];
    while let Some(user) = users.next().await {
        if let Ok(user) = user {
            user_vec.push(Value {
                value: UntaggedValue::string(user.username()),
                tag: tag.clone(),
            });
        }
    }
    let user_list = UntaggedValue::Table(user_vec);
    dict.insert_untagged("users", user_list);

    dict.into_value()
}

async fn disks(tag: Tag) -> Option<UntaggedValue> {
    let mut output = vec![];
    let mut partitions = disk::partitions_physical();
    while let Some(part) = partitions.next().await {
        if let Ok(part) = part {
            let mut dict = TaggedDictBuilder::with_capacity(&tag, 6);
            dict.insert_untagged(
                "device",
                UntaggedValue::string(
                    part.device()
                        .unwrap_or_else(|| OsStr::new("N/A"))
                        .to_string_lossy(),
                ),
            );

            dict.insert_untagged("type", UntaggedValue::string(part.file_system().as_str()));
            dict.insert_untagged(
                "mount",
                UntaggedValue::string(part.mount_point().to_string_lossy()),
            );

            if let Ok(usage) = disk::usage(part.mount_point().to_path_buf()).await {
                dict.insert_untagged(
                    "total",
                    UntaggedValue::bytes(usage.total().get::<information::byte>()),
                );
                dict.insert_untagged(
                    "used",
                    UntaggedValue::bytes(usage.used().get::<information::byte>()),
                );
                dict.insert_untagged(
                    "free",
                    UntaggedValue::bytes(usage.free().get::<information::byte>()),
                );
            }

            output.push(dict.into_value());
        }
    }

    if !output.is_empty() {
        Some(UntaggedValue::Table(output))
    } else {
        None
    }
}

async fn battery(tag: Tag) -> Option<UntaggedValue> {
    let mut output = vec![];

    if let Ok(manager) = battery::Manager::new() {
        if let Ok(batteries) = manager.batteries() {
            for battery in batteries {
                if let Ok(battery) = battery {
                    let mut dict = TaggedDictBuilder::new(&tag);
                    if let Some(vendor) = battery.vendor() {
                        dict.insert_untagged("vendor", UntaggedValue::string(vendor));
                    }
                    if let Some(model) = battery.model() {
                        dict.insert_untagged("model", UntaggedValue::string(model));
                    }
                    if let Some(cycles) = battery.cycle_count() {
                        dict.insert_untagged("cycles", UntaggedValue::int(cycles));
                    }
                    if let Some(time_to_full) = battery.time_to_full() {
                        dict.insert_untagged(
                            "mins to full",
                            UntaggedValue::decimal(
                                time_to_full.get::<battery::units::time::minute>(),
                            ),
                        );
                    }
                    if let Some(time_to_empty) = battery.time_to_empty() {
                        dict.insert_untagged(
                            "mins to empty",
                            UntaggedValue::decimal(
                                time_to_empty.get::<battery::units::time::minute>(),
                            ),
                        );
                    }
                    output.push(dict.into_value());
                }
            }
        }
    }

    if !output.is_empty() {
        Some(UntaggedValue::Table(output))
    } else {
        None
    }
}

async fn temp(tag: Tag) -> Option<UntaggedValue> {
    let mut output = vec![];

    let mut sensors = sensors::temperatures();
    while let Some(sensor) = sensors.next().await {
        if let Ok(sensor) = sensor {
            let mut dict = TaggedDictBuilder::new(&tag);
            dict.insert_untagged("unit", UntaggedValue::string(sensor.unit()));
            if let Some(label) = sensor.label() {
                dict.insert_untagged("label", UntaggedValue::string(label));
            }
            dict.insert_untagged(
                "temp",
                UntaggedValue::decimal(
                    sensor
                        .current()
                        .get::<thermodynamic_temperature::degree_celsius>(),
                ),
            );
            if let Some(high) = sensor.high() {
                dict.insert_untagged(
                    "high",
                    UntaggedValue::decimal(high.get::<thermodynamic_temperature::degree_celsius>()),
                );
            }
            if let Some(critical) = sensor.critical() {
                dict.insert_untagged(
                    "critical",
                    UntaggedValue::decimal(
                        critical.get::<thermodynamic_temperature::degree_celsius>(),
                    ),
                );
            }

            output.push(dict.into_value());
        }
    }

    if !output.is_empty() {
        Some(UntaggedValue::Table(output))
    } else {
        None
    }
}

async fn net(tag: Tag) -> Option<UntaggedValue> {
    let mut output = vec![];
    let mut io_counters = net::io_counters();
    while let Some(nic) = io_counters.next().await {
        if let Ok(nic) = nic {
            let mut network_idx = TaggedDictBuilder::with_capacity(&tag, 3);
            network_idx.insert_untagged("name", UntaggedValue::string(nic.interface()));
            network_idx.insert_untagged(
                "sent",
                UntaggedValue::bytes(nic.bytes_sent().get::<information::byte>()),
            );
            network_idx.insert_untagged(
                "recv",
                UntaggedValue::bytes(nic.bytes_recv().get::<information::byte>()),
            );
            output.push(network_idx.into_value());
        }
    }
    if !output.is_empty() {
        Some(UntaggedValue::Table(output))
    } else {
        None
    }
}

async fn sysinfo(tag: Tag) -> Vec<Value> {
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

    sysinfo.insert_value("host", host);
    if let Some(cpu) = cpu {
        sysinfo.insert_value("cpu", cpu);
    }
    if let Some(disks) = disks {
        sysinfo.insert_untagged("disks", disks);
    }
    sysinfo.insert_value("mem", memory);
    if let Some(temp) = temp {
        sysinfo.insert_untagged("temp", temp);
    }
    if let Some(net) = net {
        sysinfo.insert_untagged("net", net);
    }
    if let Some(battery) = battery {
        sysinfo.insert_untagged("battery", battery);
    }

    vec![sysinfo.into_value()]
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

    fn filter(&mut self, _: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}

fn main() {
    serve_plugin(&mut Sys::new());
}
