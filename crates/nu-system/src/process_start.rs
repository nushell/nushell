//! When the current process was created, for startup timing.

use std::time::Duration;

/// How long ago the operating system created the current process.
///
/// Nushell's startup timer starts at the top of `main`, which misses the time the loader spends
/// mapping the binary and its libraries and running the Rust runtime setup. Where the OS records
/// the process creation time precisely enough, this returns that gap so `$nu.startup-time` can
/// include it.
///
/// Returns `None` where no precise creation time is available (Linux only records it in clock
/// ticks, typically 10ms) or when the query fails.
pub fn time_since_process_start() -> Option<Duration> {
    let started = process_start_time()?;
    std::time::SystemTime::now().duration_since(started).ok()
}

#[cfg(target_os = "macos")]
fn process_start_time() -> Option<std::time::SystemTime> {
    use libproc::libproc::{bsd_info::BSDInfo, proc_pid::pidinfo};

    let pid = i32::try_from(std::process::id()).ok()?;
    let info = pidinfo::<BSDInfo>(pid, 0).ok()?;
    let since_epoch =
        Duration::from_secs(info.pbi_start_tvsec) + Duration::from_micros(info.pbi_start_tvusec);
    Some(std::time::UNIX_EPOCH + since_epoch)
}

#[cfg(target_os = "windows")]
fn process_start_time() -> Option<std::time::SystemTime> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    let mut start = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: the pseudo handle from `GetCurrentProcess` is always valid, and every out-pointer
    // refers to a live local.
    unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut start,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    }
    .ok()?;

    // FILETIME counts 100ns intervals since 1601-01-01, which is 11_644_473_600 seconds before
    // the Unix epoch.
    const EPOCH_DIFF_100NS: u64 = 11_644_473_600 * 10_000_000;
    let ticks = (u64::from(start.dwHighDateTime) << 32) | u64::from(start.dwLowDateTime);
    let since_epoch_100ns = ticks.checked_sub(EPOCH_DIFF_100NS)?;
    Some(std::time::UNIX_EPOCH + Duration::from_nanos(since_epoch_100ns.checked_mul(100)?))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn process_start_time() -> Option<std::time::SystemTime> {
    None
}
