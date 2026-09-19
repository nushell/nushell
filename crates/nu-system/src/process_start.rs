//! How long the current process had been running before `main` started, for startup timing.

use std::time::Duration;

/// How long ago the operating system created the current process.
///
/// Nushell's startup timer starts at the top of `main`, which misses the time the loader spends
/// mapping the binary and its libraries and running the Rust runtime setup. This returns that gap
/// so `$nu.startup-time` can include it. Call it as early in `main` as possible: the value is
/// "time consumed so far", not a fixed timestamp.
///
/// What is measured depends on what the platform records:
///
/// - macOS and Windows record the process creation time with microsecond (macOS) or 100ns
///   (Windows) precision, so the gap is real elapsed time.
/// - Linux and the other Unixes only record the creation time in clock ticks (usually 10ms), which
///   is coarser than the whole startup of `nu -n --no-std-lib` and always rounds the start
///   earlier, so it would inflate every measurement by a random 0-10ms. Instead the CPU time the
///   process has consumed so far (`getrusage`, microsecond precision) stands in for the gap. The
///   loader phase is almost entirely CPU work and page faults, both of which count as CPU time,
///   so this is close to the elapsed time and errs low rather than high. It does not include time
///   spent blocked, for example waiting on a cold disk for the binary's pages.
///
/// Returns `None` when the query fails.
pub fn time_since_process_start() -> Option<Duration> {
    imp::time_since_process_start()
}

#[cfg(target_os = "macos")]
mod imp {
    use super::Duration;
    use libproc::libproc::{bsd_info::BSDInfo, proc_pid::pidinfo};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub(super) fn time_since_process_start() -> Option<Duration> {
        let pid = i32::try_from(std::process::id()).ok()?;
        let info = pidinfo::<BSDInfo>(pid, 0).ok()?;
        let started = UNIX_EPOCH
            + Duration::from_secs(info.pbi_start_tvsec)
            + Duration::from_micros(info.pbi_start_tvusec);
        SystemTime::now().duration_since(started).ok()
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    pub(super) fn time_since_process_start() -> Option<Duration> {
        let mut start = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        // SAFETY: the pseudo handle from `GetCurrentProcess` is always valid, and every
        // out-pointer refers to a live local.
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

        // FILETIME counts 100ns intervals since 1601-01-01, which is 11_644_473_600 seconds
        // before the Unix epoch.
        const EPOCH_DIFF_100NS: u64 = 11_644_473_600 * 10_000_000;
        let ticks = (u64::from(start.dwHighDateTime) << 32) | u64::from(start.dwLowDateTime);
        let since_epoch_100ns = ticks.checked_sub(EPOCH_DIFF_100NS)?;
        let started = UNIX_EPOCH + Duration::from_nanos(since_epoch_100ns.checked_mul(100)?);
        SystemTime::now().duration_since(started).ok()
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::Duration;

    /// See the module documentation for why the kernel's own start time is not used here.
    pub(super) fn time_since_process_start() -> Option<Duration> {
        super::cpu_time_so_far()
    }
}

/// CPU time (user + system) consumed by the process so far. Shared by every Unix so the code is
/// built and tested on all of them, even where it is not the primary source.
#[cfg(unix)]
#[cfg_attr(
    target_os = "macos",
    allow(
        dead_code,
        reason = "macOS uses the precise creation time; kept for the shared test"
    )
)]
fn cpu_time_so_far() -> Option<Duration> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: `RUSAGE_SELF` is always a valid target and `usage` points at writable memory of the
    // right size; the kernel fills it in on success.
    let ret = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if ret != 0 {
        return None;
    }
    // SAFETY: `getrusage` returned 0, so the struct is fully initialized.
    let usage = unsafe { usage.assume_init() };
    let timeval = |tv: libc::timeval| {
        Some(
            Duration::from_secs(u64::try_from(tv.tv_sec).ok()?)
                + Duration::from_micros(u64::try_from(tv.tv_usec).ok()?),
        )
    };
    Some(timeval(usage.ru_utime)? + timeval(usage.ru_stime)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_since_process_start_is_small_and_positive() {
        let gap = time_since_process_start().expect("query should succeed on this platform");
        assert!(gap > Duration::ZERO, "gap was {gap:?}");
        assert!(gap < Duration::from_secs(60), "gap was {gap:?}");
    }

    #[cfg(unix)]
    #[test]
    fn cpu_time_grows_with_work() {
        let before = cpu_time_so_far().expect("getrusage should succeed");
        // Burn a little CPU so the counter moves.
        let mut acc = 0u64;
        for i in 0..5_000_000u64 {
            acc = acc.wrapping_mul(31).wrapping_add(i);
        }
        std::hint::black_box(acc);
        let after = cpu_time_so_far().expect("getrusage should succeed");
        assert!(after > before, "before {before:?}, after {after:?}");
    }
}

#[cfg(not(any(unix, target_os = "windows")))]
mod imp {
    use super::Duration;

    pub(super) fn time_since_process_start() -> Option<Duration> {
        None
    }
}
