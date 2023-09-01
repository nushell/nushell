use sysinfo::SystemExt;

pub fn get_os_name() -> &'static str {
    std::env::consts::OS
}

pub fn get_os_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn get_os_family() -> &'static str {
    std::env::consts::FAMILY
}

pub fn get_kernel_version() -> String {
    let sys = sysinfo::System::new();
    match sys.kernel_version() {
        Some(v) => v,
        None => "unknown".to_string(),
    }
}
