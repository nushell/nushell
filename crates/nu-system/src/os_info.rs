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
    sysinfo::System::kernel_version().unwrap_or_else(|_| "unknown".to_string())
}
