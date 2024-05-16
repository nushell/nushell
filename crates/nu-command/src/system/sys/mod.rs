mod cpu;
mod disks;
mod host;
mod mem;
mod net;
mod sys_;
mod temp;
mod users;

pub use cpu::SysCpu;
pub use disks::SysDisks;
pub use host::SysHost;
pub use mem::SysMem;
pub use net::SysNet;
pub use sys_::Sys;
pub use temp::SysTemp;
pub use users::SysUsers;

use chrono::{DateTime, FixedOffset, Local};
use nu_protocol::{record, Record, Span, Value};
use sysinfo::{
    Components, CpuRefreshKind, Disks, Networks, System, Users, MINIMUM_CPU_UPDATE_INTERVAL,
};

pub fn trim_cstyle_null(s: impl AsRef<str>) -> String {
    s.as_ref().trim_matches('\0').into()
}

pub fn disks(span: Span) -> Value {
    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| {
            let device = trim_cstyle_null(disk.name().to_string_lossy());
            let typ = trim_cstyle_null(disk.file_system().to_string_lossy());

            let record = record! {
                "device" => Value::string(device, span),
                "type" => Value::string(typ, span),
                "mount" => Value::string(disk.mount_point().to_string_lossy(), span),
                "total" => Value::filesize(disk.total_space() as i64, span),
                "free" => Value::filesize(disk.available_space() as i64, span),
                "removable" => Value::bool(disk.is_removable(), span),
                "kind" => Value::string(disk.kind().to_string(), span),
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(disks, span)
}

pub fn net(span: Span) -> Value {
    let networks = Networks::new_with_refreshed_list()
        .iter()
        .map(|(iface, data)| {
            let record = record! {
                "name" => Value::string(trim_cstyle_null(iface), span),
                "sent" => Value::filesize(data.total_transmitted() as i64, span),
                "recv" => Value::filesize(data.total_received() as i64, span),
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(networks, span)
}

pub fn cpu(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_cpu_specifics(CpuRefreshKind::everything());
    // We must refresh the CPU twice a while apart to get valid usage data.
    // In theory we could just sleep MINIMUM_CPU_UPDATE_INTERVAL, but I've noticed that
    // that gives poor results (error of ~5%). Decided to wait 2x that long, somewhat arbitrarily
    std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL * 2);
    sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());

    let cpus = sys
        .cpus()
        .iter()
        .map(|cpu| {
            // sysinfo CPU usage numbers are not very precise unless you wait a long time between refreshes.
            // Round to 1DP (chosen somewhat arbitrarily) so people aren't misled by high-precision floats.
            let rounded_usage = (cpu.cpu_usage() * 10.0).round() / 10.0;

            let load_avg = System::load_average();
            let load_avg = format!(
                "{:.2}, {:.2}, {:.2}",
                load_avg.one, load_avg.five, load_avg.fifteen
            );

            let record = record! {
                "name" => Value::string(trim_cstyle_null(cpu.name()), span),
                "brand" => Value::string(trim_cstyle_null(cpu.brand()), span),
                "freq" => Value::int(cpu.frequency() as i64, span),
                "cpu_usage" => Value::float(rounded_usage.into(), span),
                "load_average" => Value::string(load_avg, span),
                "vendor_id" => Value::string(trim_cstyle_null(cpu.vendor_id()), span),
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(cpus, span)
}

pub fn mem(span: Span) -> Value {
    let mut sys = System::new();
    sys.refresh_memory();

    let record = record! {
        "total" => Value::filesize(sys.total_memory() as i64, span),
        "free" => Value::filesize(sys.free_memory() as i64, span),
        "used" => Value::filesize(sys.used_memory() as i64, span),
        "available" => Value::filesize(sys.available_memory() as i64, span),
        "swap total" => Value::filesize(sys.total_swap() as i64, span),
        "swap free" => Value::filesize(sys.free_swap() as i64, span),
        "swap used" => Value::filesize(sys.used_swap() as i64, span),
    };

    Value::record(record, span)
}

pub fn users(span: Span) -> Value {
    let users = Users::new_with_refreshed_list()
        .iter()
        .map(|user| {
            let groups = user
                .groups()
                .iter()
                .map(|group| Value::string(trim_cstyle_null(group.name()), span))
                .collect();

            let record = record! {
                "name" => Value::string(trim_cstyle_null(user.name()), span),
                "groups" => Value::list(groups, span),
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(users, span)
}

pub fn host(span: Span) -> Record {
    let mut record = Record::new();

    if let Some(name) = System::name() {
        record.push("name", Value::string(trim_cstyle_null(name), span));
    }
    if let Some(version) = System::os_version() {
        record.push("os_version", Value::string(trim_cstyle_null(version), span));
    }
    if let Some(long_version) = System::long_os_version() {
        record.push(
            "long_os_version",
            Value::string(trim_cstyle_null(long_version), span),
        );
    }
    if let Some(version) = System::kernel_version() {
        record.push(
            "kernel_version",
            Value::string(trim_cstyle_null(version), span),
        );
    }
    if let Some(hostname) = System::host_name() {
        record.push("hostname", Value::string(trim_cstyle_null(hostname), span));
    }

    let uptime = System::uptime()
        .saturating_mul(1_000_000_000)
        .try_into()
        .unwrap_or(i64::MAX);

    record.push("uptime", Value::duration(uptime, span));

    let boot_time = boot_time()
        .map(|time| Value::date(time, span))
        .unwrap_or(Value::nothing(span));

    record.push("boot_time", boot_time);

    record
}

fn boot_time() -> Option<DateTime<FixedOffset>> {
    // Broken systems can apparently return really high values.
    // See: https://github.com/nushell/nushell/issues/10155
    // First, try to convert u64 to i64, and then try to create a `DateTime`.
    let secs = System::boot_time().try_into().ok()?;
    let time = DateTime::from_timestamp(secs, 0)?;
    Some(time.with_timezone(&Local).fixed_offset())
}

pub fn temp(span: Span) -> Value {
    let components = Components::new_with_refreshed_list()
        .iter()
        .map(|component| {
            let mut record = record! {
                "unit" => Value::string(component.label(), span),
                "temp" => Value::float(component.temperature().into(), span),
                "high" => Value::float(component.max().into(), span),
            };

            if let Some(critical) = component.critical() {
                record.push("critical", Value::float(critical.into(), span));
            }

            Value::record(record, span)
        })
        .collect();

    Value::list(components, span)
}
