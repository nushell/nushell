use crate::errors::ShellError;
use crate::object::base::OF64;
use crate::object::Dictionary;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use sys_info::*;
use sysinfo::{ComponentExt, DiskExt, NetworkExt, SystemExt};

pub fn sysinfo(_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut idx = indexmap::IndexMap::new();

    if let (Ok(name), Ok(version)) = (os_type(), os_release()) {
        let mut os_idx = indexmap::IndexMap::new();
        os_idx.insert(
            "name".to_string(),
            Value::Primitive(Primitive::String(name)),
        );
        os_idx.insert(
            "version".to_string(),
            Value::Primitive(Primitive::String(version)),
        );

        idx.insert("os".to_string(), Value::Object(Dictionary::from(os_idx)));
    }

    if let (Ok(num_cpu), Ok(cpu_speed)) = (cpu_num(), cpu_speed()) {
        let mut cpu_idx = indexmap::IndexMap::new();
        cpu_idx.insert(
            "num".to_string(),
            Value::Primitive(Primitive::Int(num_cpu as i64)),
        );
        cpu_idx.insert(
            "speed".to_string(),
            Value::Primitive(Primitive::Int(cpu_speed as i64)),
        );

        idx.insert("cpu".to_string(), Value::Object(Dictionary::from(cpu_idx)));
    }

    if let Ok(x) = loadavg() {
        let mut load_idx = indexmap::IndexMap::new();
        load_idx.insert(
            "1min".to_string(),
            Value::Primitive(Primitive::Float(OF64::from(x.one))),
        );
        load_idx.insert(
            "5min".to_string(),
            Value::Primitive(Primitive::Float(OF64::from(x.five))),
        );
        load_idx.insert(
            "15min".to_string(),
            Value::Primitive(Primitive::Float(OF64::from(x.fifteen))),
        );

        idx.insert(
            "load avg".to_string(),
            Value::Object(Dictionary::from(load_idx)),
        );
    }

    if let Ok(x) = mem_info() {
        let mut mem_idx = indexmap::IndexMap::new();
        mem_idx.insert(
            "total".to_string(),
            Value::Primitive(Primitive::Bytes(x.total as u128 * 1024)),
        );
        mem_idx.insert(
            "free".to_string(),
            Value::Primitive(Primitive::Bytes(x.free as u128 * 1024)),
        );
        mem_idx.insert(
            "avail".to_string(),
            Value::Primitive(Primitive::Bytes(x.avail as u128 * 1024)),
        );
        mem_idx.insert(
            "buffers".to_string(),
            Value::Primitive(Primitive::Bytes(x.buffers as u128 * 1024)),
        );
        mem_idx.insert(
            "cached".to_string(),
            Value::Primitive(Primitive::Bytes(x.cached as u128 * 1024)),
        );
        mem_idx.insert(
            "swap total".to_string(),
            Value::Primitive(Primitive::Bytes(x.swap_total as u128 * 1024)),
        );
        mem_idx.insert(
            "swap free".to_string(),
            Value::Primitive(Primitive::Bytes(x.swap_free as u128 * 1024)),
        );

        idx.insert("mem".to_string(), Value::Object(Dictionary::from(mem_idx)));
    }

    /*
    if let Ok(x) = disk_info() {
        let mut disk_idx = indexmap::IndexMap::new();
        disk_idx.insert(
            "total".to_string(),
            Value::Primitive(Primitive::Bytes(x.total as u128 * 1024)),
        );
        disk_idx.insert(
            "free".to_string(),
            Value::Primitive(Primitive::Bytes(x.free as u128 * 1024)),
        );
    }
    */

    if let Ok(x) = hostname() {
        idx.insert(
            "hostname".to_string(),
            Value::Primitive(Primitive::String(x)),
        );
    }

    #[cfg(not(windows))]
    {
        if let Ok(x) = boottime() {
            let mut boottime_idx = indexmap::IndexMap::new();
            boottime_idx.insert(
                "days".to_string(),
                Value::Primitive(Primitive::Int(x.tv_sec / (24 * 3600))),
            );
            boottime_idx.insert(
                "hours".to_string(),
                Value::Primitive(Primitive::Int((x.tv_sec / 3600) % 24)),
            );
            boottime_idx.insert(
                "mins".to_string(),
                Value::Primitive(Primitive::Int((x.tv_sec / 60) % 60)),
            );

            idx.insert(
                "uptime".to_string(),
                Value::Object(Dictionary::from(boottime_idx)),
            );
        }
    }

    let system = sysinfo::System::new();
    let components_list = system.get_components_list();
    if components_list.len() > 0 {
        let mut v = vec![];
        for component in components_list {
            let mut component_idx = indexmap::IndexMap::new();
            component_idx.insert(
                "name".to_string(),
                Value::string(component.get_label().to_string()),
            );
            component_idx.insert(
                "temp".to_string(),
                Value::float(component.get_temperature() as f64),
            );
            component_idx.insert(
                "max".to_string(),
                Value::float(component.get_max() as f64),
            );
            if let Some(critical) = component.get_critical() {
                component_idx.insert("critical".to_string(), Value::float(critical as f64));
            }
            v.push(Value::Object(Dictionary::from(component_idx)));
        }
        idx.insert("temps".to_string(), Value::List(v));
    }

    let disks = system.get_disks();
    if disks.len() > 0 {
        let mut v = vec![];

        for disk in disks {
            let mut disk_idx = indexmap::IndexMap::new();
            disk_idx.insert(
                "name".to_string(),
                Value::string(disk.get_name().to_string_lossy()),
            );
            disk_idx.insert(
                "available".to_string(),
                Value::bytes(disk.get_available_space()),
            );
            disk_idx.insert(
                "total".to_string(),
                Value::bytes(disk.get_total_space()),
            );
            v.push(Value::Object(Dictionary::from(disk_idx)));
        }

        idx.insert("disks".to_string(), Value::List(v));
    }

    let network = system.get_network();
    let incoming = network.get_income();
    let outgoing = network.get_outcome();

    let mut network_idx = indexmap::IndexMap::new();
    network_idx.insert("incoming".to_string(), Value::bytes(incoming));
    network_idx.insert("outgoing".to_string(), Value::bytes(outgoing));
    idx.insert("network".to_string(), Value::Object(Dictionary::from(network_idx)));

    // println!("{:#?}", system.get_network());

    let mut stream = VecDeque::new();
    stream.push_back(ReturnValue::Value(Value::Object(Dictionary::from(idx))));

    Ok(stream.boxed())
}
