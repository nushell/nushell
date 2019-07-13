use crate::errors::ShellError;
use crate::object::base::OF64;
use crate::object::SpannedDictBuilder;
use crate::object::{Primitive, Value};
use crate::prelude::*;
use sys_info::*;
use sysinfo::{ComponentExt, DiskExt, NetworkExt, RefreshKind, SystemExt};

pub fn sysinfo(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut idx = SpannedDictBuilder::new(args.name_span);

    if let (Ok(name), Ok(version)) = (os_type(), os_release()) {
        let mut os_idx = SpannedDictBuilder::new(args.name_span);
        os_idx.insert("name", Primitive::String(name));
        os_idx.insert("version", Primitive::String(version));

        idx.insert_spanned("os", os_idx.into_spanned_value());
    }

    if let (Ok(num_cpu), Ok(cpu_speed)) = (cpu_num(), cpu_speed()) {
        let mut cpu_idx = SpannedDictBuilder::new(args.name_span);
        cpu_idx.insert("num", Primitive::Int(num_cpu as i64));
        cpu_idx.insert("speed", Primitive::Int(cpu_speed as i64));

        idx.insert_spanned("cpu", cpu_idx);
    }

    if let Ok(x) = loadavg() {
        let mut load_idx = SpannedDictBuilder::new(args.name_span);

        load_idx.insert("1min", Primitive::Float(OF64::from(x.one)));
        load_idx.insert("5min", Primitive::Float(OF64::from(x.five)));
        load_idx.insert("15min", Primitive::Float(OF64::from(x.fifteen)));

        idx.insert_spanned("load avg", load_idx);
    }

    if let Ok(x) = mem_info() {
        let mut mem_idx = SpannedDictBuilder::new(args.name_span);

        mem_idx.insert("total", Primitive::Bytes(x.total as u64 * 1024));
        mem_idx.insert("free", Primitive::Bytes(x.free as u64 * 1024));
        mem_idx.insert("avail", Primitive::Bytes(x.avail as u64 * 1024));
        mem_idx.insert("buffers", Primitive::Bytes(x.buffers as u64 * 1024));
        mem_idx.insert("cached", Primitive::Bytes(x.cached as u64 * 1024));
        mem_idx.insert("swap total", Primitive::Bytes(x.swap_total as u64 * 1024));
        mem_idx.insert("swap free", Primitive::Bytes(x.swap_free as u64 * 1024));

        idx.insert_spanned("mem", mem_idx);
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
        idx.insert("hostname", Primitive::String(x));
    }

    #[cfg(not(windows))]
    {
        if let Ok(x) = boottime() {
            let mut boottime_idx = SpannedDictBuilder::new(args.name_span);
            boottime_idx.insert("days", Primitive::Int(x.tv_sec / (24 * 3600)));
            boottime_idx.insert("hours", Primitive::Int((x.tv_sec / 3600) % 24));
            boottime_idx.insert("mins", Primitive::Int((x.tv_sec / 60) % 60));

            idx.insert_spanned("uptime", boottime_idx);
        }
    }

    let system = sysinfo::System::new_with_specifics(RefreshKind::everything().without_processes());
    let components_list = system.get_components_list();
    if components_list.len() > 0 {
        let mut v: Vec<Spanned<Value>> = vec![];
        for component in components_list {
            let mut component_idx = SpannedDictBuilder::new(args.name_span);
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
        idx.insert("temps", Value::List(v));
    }

    let disks = system.get_disks();
    if disks.len() > 0 {
        let mut v = vec![];

        for disk in disks {
            let mut disk_idx = SpannedDictBuilder::new(args.name_span);
            disk_idx.insert("name", Value::string(disk.get_name().to_string_lossy()));
            disk_idx.insert("available", Value::bytes(disk.get_available_space()));
            disk_idx.insert("total", Value::bytes(disk.get_total_space()));
            v.push(disk_idx.into());
        }

        idx.insert("disks", Value::List(v));
    }

    let network = system.get_network();
    let incoming = network.get_income();
    let outgoing = network.get_outcome();

    let mut network_idx = SpannedDictBuilder::new(args.name_span);
    network_idx.insert("incoming", Value::bytes(incoming));
    network_idx.insert("outgoing", Value::bytes(outgoing));
    idx.insert_spanned("network", network_idx);

    let stream = stream![idx.into_spanned_value()];

    Ok(stream.from_input_stream())
}
