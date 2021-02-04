use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use sysinfo::{ComponentExt, DiskExt, NetworkExt, ProcessorExt, System, SystemExt, UserExt};

#[derive(Default)]
pub struct Sys;

impl Sys {
    pub fn new() -> Sys {
        Sys
    }
}

pub fn disks(sys: &mut System, tag: Tag) -> Option<UntaggedValue> {
    sys.refresh_disks();

    let mut output = vec![];
    for disk in sys.get_disks() {
        let mut dict = TaggedDictBuilder::new(&tag);
        dict.insert_untagged(
            "device",
            UntaggedValue::string(trim_cstyle_null(
                disk.get_name().to_string_lossy().to_string(),
            )),
        );
        dict.insert_untagged(
            "type",
            UntaggedValue::string(trim_cstyle_null(
                String::from_utf8_lossy(disk.get_file_system()).to_string(),
            )),
        );
        dict.insert_untagged("mount", UntaggedValue::filepath(disk.get_mount_point()));
        dict.insert_untagged("total", UntaggedValue::filesize(disk.get_total_space()));
        dict.insert_untagged("free", UntaggedValue::filesize(disk.get_available_space()));
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

    let mut output = vec![];
    for (iface, data) in sys.get_networks() {
        let mut dict = TaggedDictBuilder::new(&tag);
        dict.insert_untagged(
            "name",
            UntaggedValue::string(trim_cstyle_null(iface.to_string())),
        );
        dict.insert_untagged(
            "sent",
            UntaggedValue::filesize(data.get_total_transmitted()),
        );
        dict.insert_untagged("recv", UntaggedValue::filesize(data.get_total_received()));

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
    for cpu in sys.get_processors() {
        let mut dict = TaggedDictBuilder::new(&tag);
        dict.insert_untagged(
            "name",
            UntaggedValue::string(trim_cstyle_null(cpu.get_name().to_string())),
        );
        dict.insert_untagged(
            "brand",
            UntaggedValue::string(trim_cstyle_null(cpu.get_brand().to_string())),
        );
        dict.insert_untagged("freq", UntaggedValue::int(cpu.get_frequency()));

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
    let total_mem = sys.get_total_memory();
    let free_mem = sys.get_free_memory();
    let total_swap = sys.get_total_swap();
    let free_swap = sys.get_free_swap();

    dict.insert_untagged("total", UntaggedValue::filesize(total_mem * 1000));
    dict.insert_untagged("free", UntaggedValue::filesize(free_mem * 1000));
    dict.insert_untagged("swap total", UntaggedValue::filesize(total_swap * 1000));
    dict.insert_untagged("swap free", UntaggedValue::filesize(free_swap * 1000));

    Some(dict.into_untagged_value())
}

pub fn host(sys: &mut System, tag: Tag) -> Option<UntaggedValue> {
    sys.refresh_users_list();

    let mut dict = TaggedDictBuilder::new(&tag);
    if let Some(name) = sys.get_name() {
        dict.insert_untagged("name", UntaggedValue::string(trim_cstyle_null(name)));
    }
    if let Some(version) = sys.get_os_version() {
        dict.insert_untagged(
            "os version",
            UntaggedValue::string(trim_cstyle_null(version)),
        );
    }
    if let Some(version) = sys.get_kernel_version() {
        dict.insert_untagged(
            "kernel version",
            UntaggedValue::string(trim_cstyle_null(version)),
        );
    }
    if let Some(hostname) = sys.get_host_name() {
        dict.insert_untagged(
            "hostname",
            UntaggedValue::string(trim_cstyle_null(hostname)),
        );
    }
    dict.insert_untagged(
        "uptime",
        UntaggedValue::duration(1000000000 * sys.get_uptime()),
    );

    let mut users = vec![];
    for user in sys.get_users() {
        let mut user_dict = TaggedDictBuilder::new(&tag);
        user_dict.insert_untagged(
            "name",
            UntaggedValue::string(trim_cstyle_null(user.get_name().to_string())),
        );

        let mut groups = vec![];
        for group in user.get_groups() {
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

    for component in sys.get_components() {
        let mut dict = TaggedDictBuilder::new(&tag);

        dict.insert_untagged("unit", UntaggedValue::string(component.get_label()));
        dict.insert_untagged(
            "temp",
            UntaggedValue::decimal_from_float(component.get_temperature() as f64, tag.span),
        );
        dict.insert_untagged(
            "high",
            UntaggedValue::decimal_from_float(component.get_max() as f64, tag.span),
        );

        if let Some(critical) = component.get_critical() {
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

pub async fn sysinfo(tag: Tag) -> Vec<Value> {
    let mut sys = System::new_all();

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

    vec![sysinfo.into_value()]
}

pub fn trim_cstyle_null(s: String) -> String {
    s.trim_matches(char::from(0)).to_string()
}
