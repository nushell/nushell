use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue};
use sysinfo::{ComponentExt, DiskExt, NetworkExt, ProcessorExt, System, SystemExt, UserExt};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "sys"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys")
            .desc("View information about the current system.")
            .filter()
    }

    fn usage(&self) -> &str {
        "View information about the system."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_sys(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show info about the system",
            example: "sys",
            result: None,
        }]
    }
}

fn run_sys(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.name_tag();
    let mut sys = System::new();

    let mut sysinfo = TaggedDictBuilder::with_capacity(&tag, 6);

    if let Some(host) = host(&mut sys, tag.clone()) {
        sysinfo.insert_value("host", host);
    }
    if let Some(cpus) = cpu(&mut sys, tag.clone()) {
        sysinfo.insert_value("cpu", cpus);
    }
    if let Some(disks) = disks(&mut sys, tag.clone()) {
        sysinfo.insert_value("disks", disks);
    }
    if let Some(mem) = mem(&mut sys, tag.clone()) {
        sysinfo.insert_value("mem", mem);
    }
    if let Some(temp) = temp(&mut sys, tag.clone()) {
        sysinfo.insert_value("temp", temp);
    }
    if let Some(net) = net(&mut sys, tag) {
        sysinfo.insert_value("net", net);
    }

    Ok(vec![sysinfo.into_value()].into_iter().into_output_stream())
}

pub fn trim_cstyle_null(s: String) -> String {
    s.trim_matches(char::from(0)).to_string()
}

pub fn disks(sys: &mut System, tag: Tag) -> Option<UntaggedValue> {
    sys.refresh_disks();
    sys.refresh_disks_list();

    let mut output = vec![];
    for disk in sys.disks() {
        let mut dict = TaggedDictBuilder::new(&tag);
        dict.insert_untagged(
            "device",
            UntaggedValue::string(trim_cstyle_null(disk.name().to_string_lossy().to_string())),
        );
        dict.insert_untagged(
            "type",
            UntaggedValue::string(trim_cstyle_null(
                String::from_utf8_lossy(disk.file_system()).to_string(),
            )),
        );
        dict.insert_untagged("mount", UntaggedValue::filepath(disk.mount_point()));
        dict.insert_untagged("total", UntaggedValue::filesize(disk.total_space()));
        dict.insert_untagged("free", UntaggedValue::filesize(disk.available_space()));
        output.push(dict.into_value());
    }
    if !output.is_empty() {
        Some(UntaggedValue::Table(output))
    } else {
        None
    }
}

pub fn net(sys: &mut System, tag: Tag) -> Option<UntaggedValue> {
    sys.refresh_networks();
    sys.refresh_networks_list();

    let mut output = vec![];
    for (iface, data) in sys.networks() {
        let mut dict = TaggedDictBuilder::new(&tag);
        dict.insert_untagged(
            "name",
            UntaggedValue::string(trim_cstyle_null(iface.to_string())),
        );
        dict.insert_untagged("sent", UntaggedValue::filesize(data.total_transmitted()));
        dict.insert_untagged("recv", UntaggedValue::filesize(data.total_received()));

        output.push(dict.into_value());
    }
    if !output.is_empty() {
        Some(UntaggedValue::Table(output))
    } else {
        None
    }
}

pub fn cpu(sys: &mut System, tag: Tag) -> Option<UntaggedValue> {
    sys.refresh_cpu();

    let mut output = vec![];
    for cpu in sys.processors() {
        let mut dict = TaggedDictBuilder::new(&tag);
        dict.insert_untagged(
            "name",
            UntaggedValue::string(trim_cstyle_null(cpu.name().to_string())),
        );
        dict.insert_untagged(
            "brand",
            UntaggedValue::string(trim_cstyle_null(cpu.brand().to_string())),
        );
        dict.insert_untagged("freq", UntaggedValue::int(cpu.frequency() as i64));

        output.push(dict.into_value());
    }
    if !output.is_empty() {
        Some(UntaggedValue::Table(output))
    } else {
        None
    }
}

pub fn mem(sys: &mut System, tag: Tag) -> Option<UntaggedValue> {
    sys.refresh_memory();

    let mut dict = TaggedDictBuilder::new(tag);
    let total_mem = sys.total_memory();
    let free_mem = sys.free_memory();
    let total_swap = sys.total_swap();
    let free_swap = sys.free_swap();

    dict.insert_untagged("total", UntaggedValue::filesize(total_mem * 1000));
    dict.insert_untagged("free", UntaggedValue::filesize(free_mem * 1000));
    dict.insert_untagged("swap total", UntaggedValue::filesize(total_swap * 1000));
    dict.insert_untagged("swap free", UntaggedValue::filesize(free_swap * 1000));

    Some(dict.into_untagged_value())
}

pub fn host(sys: &mut System, tag: Tag) -> Option<UntaggedValue> {
    sys.refresh_users_list();

    let mut dict = TaggedDictBuilder::new(&tag);
    if let Some(name) = sys.name() {
        dict.insert_untagged("name", UntaggedValue::string(trim_cstyle_null(name)));
    }
    if let Some(version) = sys.os_version() {
        dict.insert_untagged(
            "os version",
            UntaggedValue::string(trim_cstyle_null(version)),
        );
    }
    if let Some(version) = sys.kernel_version() {
        dict.insert_untagged(
            "kernel version",
            UntaggedValue::string(trim_cstyle_null(version)),
        );
    }
    if let Some(hostname) = sys.host_name() {
        dict.insert_untagged(
            "hostname",
            UntaggedValue::string(trim_cstyle_null(hostname)),
        );
    }
    dict.insert_untagged(
        "uptime",
        UntaggedValue::duration(1000000000 * sys.uptime() as i64),
    );

    let mut users = vec![];
    for user in sys.users() {
        let mut user_dict = TaggedDictBuilder::new(&tag);
        user_dict.insert_untagged(
            "name",
            UntaggedValue::string(trim_cstyle_null(user.name().to_string())),
        );

        let mut groups = vec![];
        for group in user.groups() {
            groups
                .push(UntaggedValue::string(trim_cstyle_null(group.to_string())).into_value(&tag));
        }
        user_dict.insert_untagged("groups", UntaggedValue::Table(groups));

        users.push(user_dict.into_value());
    }
    if !users.is_empty() {
        dict.insert_untagged("sessions", UntaggedValue::Table(users));
    }

    Some(dict.into_untagged_value())
}

pub fn temp(sys: &mut System, tag: Tag) -> Option<UntaggedValue> {
    sys.refresh_components();
    sys.refresh_components_list();

    let mut output = vec![];

    for component in sys.components() {
        let mut dict = TaggedDictBuilder::new(&tag);

        dict.insert_untagged("unit", UntaggedValue::string(component.label()));
        dict.insert_untagged(
            "temp",
            UntaggedValue::decimal_from_float(component.temperature() as f64, tag.span),
        );
        dict.insert_untagged(
            "high",
            UntaggedValue::decimal_from_float(component.max() as f64, tag.span),
        );

        if let Some(critical) = component.critical() {
            dict.insert_untagged(
                "critical",
                UntaggedValue::decimal_from_float(critical as f64, tag.span),
            );
        }
        output.push(dict.into_value());
    }
    if !output.is_empty() {
        Some(UntaggedValue::Table(output))
    } else {
        None
    }
}
