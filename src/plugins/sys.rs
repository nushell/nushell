#![feature(async_await)]

use futures::executor::block_on;
use futures::stream::StreamExt;
use heim::{disk, memory};
use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, CommandConfig, Plugin, Primitive, ReturnSuccess, ReturnValue,
    ShellError, Span, Spanned, SpannedDictBuilder, Value, OF64,
};
use std::ffi::OsStr;

struct Sys;
impl Sys {
    fn new() -> Sys {
        Sys
    }
}

//TODO: add more error checking

async fn cpu(span: Span) -> Option<Spanned<Value>> {
    if let (Ok(num_cpu), Ok(cpu_speed)) = (sys_info::cpu_num(), sys_info::cpu_speed()) {
        let mut cpu_idx = SpannedDictBuilder::new(span);
        cpu_idx.insert("cores", Primitive::Int(num_cpu as i64));
        cpu_idx.insert("speed", Primitive::Int(cpu_speed as i64));
        Some(cpu_idx.into_spanned_value())
    } else {
        None
    }
}

async fn mem(span: Span) -> Spanned<Value> {
    let mut dict = SpannedDictBuilder::new(span);

    if let Ok(memory) = memory::memory().await {
        dict.insert("total", Value::bytes(memory.total().get()));
        dict.insert("free", Value::bytes(memory.free().get()));
    }
    if let Ok(swap) = memory::swap().await {
        dict.insert("swap total", Value::bytes(swap.total().get()));
        dict.insert("swap free", Value::bytes(swap.free().get()));
    }

    dict.into_spanned_value()
}

async fn host(span: Span) -> Spanned<Value> {
    let mut dict = SpannedDictBuilder::new(span);

    // OS
    if let Ok(platform) = heim::host::platform().await {
        dict.insert("name", Value::string(platform.system()));
        dict.insert("release", Value::string(platform.release()));
        dict.insert("hostname", Value::string(platform.hostname()));
        dict.insert("arch", Value::string(platform.architecture().as_str()));
    }

    // Uptime
    if let Ok(uptime) = heim::host::uptime().await {
        let mut uptime_dict = SpannedDictBuilder::new(span);

        let uptime = uptime.get().round() as i64;
        let days = uptime / (60 * 60 * 24);
        let hours = (uptime - days * 60 * 60 * 24) / (60 * 60);
        let minutes = (uptime - days * 60 * 60 * 24 - hours * 60 * 60) / 60;
        let seconds = uptime % 60;

        uptime_dict.insert("days", Value::int(days));
        uptime_dict.insert("hours", Value::int(hours));
        uptime_dict.insert("mins", Value::int(minutes));
        uptime_dict.insert("secs", Value::int(seconds));

        dict.insert_spanned("uptime", uptime_dict.into_spanned_value());
    }

    // Users
    let mut users = heim::host::users();
    let mut user_vec = vec![];
    while let Some(user) = users.next().await {
        if let Ok(user) = user {
            user_vec.push(Spanned {
                item: Value::string(user.username()),
                span,
            });
        }
    }
    let user_list = Value::List(user_vec);
    dict.insert("users", user_list);

    dict.into_spanned_value()
}

async fn disks(span: Span) -> Value {
    let mut output = vec![];
    let mut partitions = disk::partitions_physical();
    while let Some(part) = partitions.next().await {
        if let Ok(part) = part {
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
            if let Ok(usage) = disk::usage(part.mount_point().to_path_buf()).await {
                dict.insert("total", Value::bytes(usage.total().get()));
                dict.insert("used", Value::bytes(usage.used().get()));
                dict.insert("free", Value::bytes(usage.free().get()));
            }
            output.push(dict.into_spanned_value());
        }
    }

    Value::List(output)
}

async fn temp(span: Span) -> Value {
    use sysinfo::{ComponentExt, RefreshKind, SystemExt};
    let system = sysinfo::System::new_with_specifics(RefreshKind::new().with_system());
    let components_list = system.get_components_list();
    if components_list.len() > 0 {
        let mut v: Vec<Spanned<Value>> = vec![];
        for component in components_list {
            let mut component_idx = SpannedDictBuilder::new(span);
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

async fn net(span: Span) -> Spanned<Value> {
    use sysinfo::{NetworkExt, RefreshKind, SystemExt};
    let system = sysinfo::System::new_with_specifics(RefreshKind::new().with_network());

    let network = system.get_network();
    let incoming = network.get_income();
    let outgoing = network.get_outcome();

    let mut network_idx = SpannedDictBuilder::new(span);
    network_idx.insert("incoming", Value::bytes(incoming));
    network_idx.insert("outgoing", Value::bytes(outgoing));
    network_idx.into_spanned_value()
}

async fn sysinfo(span: Span) -> Vec<Spanned<Value>> {
    let mut sysinfo = SpannedDictBuilder::new(span);

    sysinfo.insert_spanned("host", host(span).await);
    if let Some(cpu) = cpu(span).await {
        sysinfo.insert_spanned("cpu", cpu);
    }
    sysinfo.insert("disks", disks(span).await);
    sysinfo.insert_spanned("mem", mem(span).await);
    sysinfo.insert("temp", temp(span).await);
    sysinfo.insert_spanned("net", net(span).await);

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
        Ok(block_on(sysinfo(
            callinfo.name_span.unwrap_or_else(|| Span::unknown()),
        ))
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
