use crate::errors::ShellError;
use crate::object::{Value, Primitive};
use crate::object::Dictionary;
use crate::object::base::OF64;
use crate::prelude::*;
use sys_info::*;

pub fn sysinfo(_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut idx = indexmap::IndexMap::new();

    if let (Ok(name), Ok(version)) = (os_type(), os_release()) {
        let mut os_idx = indexmap::IndexMap::new();
        os_idx.insert("name".to_string(), Value::Primitive(Primitive::String(name)));
        os_idx.insert("version".to_string(), Value::Primitive(Primitive::String(version)));

        idx.insert("os".to_string(), Value::Object(Dictionary::from(os_idx)));
    }

    if let (Ok(num_cpu), Ok(cpu_speed)) = (cpu_num(), cpu_speed()) {
        let mut cpu_idx = indexmap::IndexMap::new();
        cpu_idx.insert("num".to_string(), Value::Primitive(Primitive::Int(num_cpu as i64)));
        cpu_idx.insert("speed".to_string(), Value::Primitive(Primitive::Int(cpu_speed as i64)));

        idx.insert("cpu".to_string(), Value::Object(Dictionary::from(cpu_idx)));
    }

    if let Ok(x) = loadavg() {
        let mut load_idx = indexmap::IndexMap::new();
        load_idx.insert("1min".to_string(), Value::Primitive(Primitive::Float(OF64::from(x.one))));
        load_idx.insert("5min".to_string(), Value::Primitive(Primitive::Float(OF64::from(x.five))));
        load_idx.insert("15min".to_string(), Value::Primitive(Primitive::Float(OF64::from(x.fifteen))));

        idx.insert("load avg".to_string(), Value::Object(Dictionary::from(load_idx)));
    }

    if let Ok(x) = mem_info() {
        let mut mem_idx = indexmap::IndexMap::new();
        mem_idx.insert("total".to_string(), Value::Primitive(Primitive::Bytes(x.total as u128 * 1024)));
        mem_idx.insert("free".to_string(), Value::Primitive(Primitive::Bytes(x.free as u128 * 1024)));
        mem_idx.insert("avail".to_string(), Value::Primitive(Primitive::Bytes(x.avail as u128 * 1024)));
        mem_idx.insert("buffers".to_string(), Value::Primitive(Primitive::Bytes(x.buffers as u128 * 1024)));
        mem_idx.insert("cached".to_string(), Value::Primitive(Primitive::Bytes(x.cached as u128 * 1024)));
        mem_idx.insert("swap total".to_string(), Value::Primitive(Primitive::Bytes(x.swap_total as u128 * 1024)));
        mem_idx.insert("swap free".to_string(), Value::Primitive(Primitive::Bytes(x.swap_free as u128 * 1024)));

        idx.insert("mem".to_string(), Value::Object(Dictionary::from(mem_idx)));
    }

    if let Ok(x) = disk_info() {
        let mut disk_idx = indexmap::IndexMap::new();
        disk_idx.insert("total".to_string(), Value::Primitive(Primitive::Bytes(x.total as u128 * 1024)));
        disk_idx.insert("free".to_string(), Value::Primitive(Primitive::Bytes(x.free as u128 * 1024)));
    }

    if let Ok(x) = hostname() {
        idx.insert("hostname".to_string(), Value::Primitive(Primitive::String(x)));
    }    

    if let Ok(x) = boottime() {
        let mut boottime_idx = indexmap::IndexMap::new();
        boottime_idx.insert("days".to_string(), Value::Primitive(Primitive::Int(x.tv_sec / (24 * 3600))));
        boottime_idx.insert("hours".to_string(), Value::Primitive(Primitive::Int((x.tv_sec / 3600) % 24)));
        boottime_idx.insert("mins".to_string(), Value::Primitive(Primitive::Int((x.tv_sec / 60) % 60)));

        idx.insert("uptime".to_string(), Value::Object(Dictionary::from(boottime_idx)));
    }

    let mut stream = VecDeque::new();
    stream.push_back(ReturnValue::Value(Value::Object(Dictionary::from(idx))));

    Ok(stream.boxed())
}