use crate::process::{ProcessInfo, command_line};
use crate::unix::UserNames;
use log::info;
use nu_utils::time::Instant;
use procfs::process::{Process, Stat, Status};
use procfs::{FromBufRead, ProcResult, WithCurrentSystemInfo};
use std::io::Read;
use std::mem::MaybeUninit;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

/// The kernel keeps at most this many bytes of a process name (`TASK_COMM_LEN - 1`).
const COMM_MAX_LEN: usize = 15;

/// One reading of the CPU time a process has used.
struct CpuSample {
    /// Process start time in clock ticks since boot, to detect a reused pid.
    starttime: u64,
    /// `utime + stime` from `/proc/<pid>/stat`, in clock ticks (usually 10ms each).
    ticks: u64,
    /// The process's CPU-time clock. `None` when the kernel can't read it.
    clock: Option<Duration>,
}

impl CpuSample {
    fn read(stat: &Stat) -> Self {
        Self {
            starttime: stat.starttime,
            ticks: stat.utime + stat.stime,
            clock: process_cpu_time(stat.pid),
        }
    }

    /// CPU time used between `prev` and `self`, or `None` if they are different processes.
    fn since(&self, prev: &CpuSample) -> Option<Duration> {
        if self.starttime != prev.starttime {
            return None;
        }
        if let (Some(curr), Some(prev)) = (self.clock, prev.clock) {
            return Some(curr.saturating_sub(prev));
        }
        let ticks = self.ticks.saturating_sub(prev.ticks);
        Some(Duration::from_secs_f64(
            ticks as f64 / procfs::ticks_per_second() as f64,
        ))
    }
}

/// Reads the CPU time a process has used from its CPU-time clock.
///
/// `/proc/<pid>/stat` counts CPU time in clock ticks, which is too coarse for a 100ms sample: a
/// process could only ever show 0%, 10%, 20%... The clock counts nanoseconds and, like `stat`,
/// includes the time of threads that have exited. Linux lets any process read the CPU-time
/// clock of any other process.
fn process_cpu_time(pid: i32) -> Option<Duration> {
    // The id `clock_getcpuclockid(3)` returns for `pid`: the kernel's
    // `MAKE_PROCESS_CPUCLOCK(pid, CPUCLOCK_SCHED)`, which glibc, musl and bionic all compute
    // this way. Calling `clock_getcpuclockid` would cost another syscall to validate the id,
    // and Android only has it from API level 23.
    let clock: libc::clockid_t = (!pid << 3) | 2;
    let mut time = MaybeUninit::<libc::timespec>::uninit();
    // SAFETY: `time` is valid for `clock_gettime` to write a `timespec` to.
    if unsafe { libc::clock_gettime(clock, time.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: `clock_gettime` succeeded, so it filled in `time`.
    let time = unsafe { time.assume_init() };
    Some(Duration::new(
        time.tv_sec.try_into().ok()?,
        time.tv_nsec.try_into().ok()?,
    ))
}

/// Reads `/proc/<pid>/status`. The kernel writes the process name into it as raw bytes, which
/// aren't valid UTF-8 when the name was cut in the middle of a character, and procfs rejects
/// the whole file then, so decode it lossily first.
fn read_status(proc: &Process) -> ProcResult<Status> {
    let mut buf = Vec::new();
    proc.open_relative("status")?.read_to_end(&mut buf)?;
    Status::from_buf_read(String::from_utf8_lossy(&buf).as_bytes())
}

/// Reads `/proc/<pid>/environ` as `KEY=value` strings in their original order.
fn read_environ(proc: &Process) -> ProcResult<Vec<String>> {
    let mut buf = Vec::new();
    proc.open_relative("environ")?.read_to_end(&mut buf)?;
    Ok(crate::unix::split_nul(&buf))
}

/// Lists every process, measuring CPU usage over `interval`. When `long` is false, the details
/// only `ps --long` shows aren't read, so they are `None`. The command line and executable are
/// still read when the name needs them.
///
/// The executable, I/O counts, working directory and environment are only readable for the
/// current user's processes (or as root).
pub fn collect_proc(interval: Duration, long: bool) -> Vec<ProcessInfo> {
    let mut base_procs = Vec::new();

    // Take an initial snapshot of CPU usage, so we can calculate changes over time. Only keep
    // the pid: holding every `Process` open across the sleep can exhaust file descriptors.
    if let Ok(all_proc) = procfs::process::all_processes() {
        for proc in all_proc.flatten() {
            let sample = proc.stat().ok().map(|stat| CpuSample::read(&stat));
            base_procs.push((proc.pid(), sample, Instant::now()));
        }
    }

    // wait a bit...
    thread::sleep(interval);

    // now get process info again, build up results
    let mut user_names = UserNames::default();
    base_procs
        .into_iter()
        .filter_map(|(pid, prev_sample, prev_time)| {
            let Some((proc, stat)) = Process::new(pid)
                .and_then(|proc| proc.stat().map(|stat| (proc, stat)))
                .ok()
            else {
                info!(
                    "failed to retrieve info for pid={pid}, process probably died between snapshots"
                );
                return None;
            };
            let curr_sample = CpuSample::read(&stat);
            let interval = Instant::now().saturating_duration_since(prev_time);
            let cpu_usage = prev_sample
                .and_then(|prev| curr_sample.since(&prev))
                .map(|used| used.as_secs_f64() * 100.0 / interval.as_secs_f64());

            let status = read_status(&proc).ok();
            // The name restores a name the kernel cut short from the command line or executable.
            let read_paths = long || stat.comm.len() == COMM_MAX_LEN;
            let cmdline = read_paths.then(|| proc.cmdline().ok()).flatten();
            let exe = read_paths.then(|| proc.exe().ok()).flatten();
            let argv0 = cmdline.as_ref().and_then(|cmd| cmd.first()).map(Path::new);
            let name = untruncated_name(&stat.comm, [argv0, exe.as_deref()]);
            let io = long.then(|| proc.io().ok()).flatten();
            let ticks = stat.utime + stat.stime;

            Some(ProcessInfo {
                pid,
                ppid: stat.ppid,
                name,
                // Kernel threads like kworker/0:0 have an empty command line.
                command: cmdline.filter(|_| long).and_then(|cmd| command_line(&cmd)),
                exe: exe.map(|exe| exe.display().to_string()),
                user: status
                    .as_ref()
                    .and_then(|status| user_names.get(status.euid)),
                user_id: status.as_ref().map(|status| status.euid),
                status: Some(process_status(stat.state)),
                cpu_usage,
                cpu_time: Some(Duration::from_secs_f64(
                    ticks as f64 / procfs::ticks_per_second() as f64,
                )),
                mem_size: Some(stat.rss_bytes().get()),
                virtual_size: Some(stat.vsize),
                // Resident memory that isn't shared with other processes (`RssAnon`)
                private_size: status
                    .as_ref()
                    .and_then(|status| status.rssanon)
                    .map(|kib| kib.saturating_mul(1024)),
                disk_read: io.as_ref().map(|io| io.read_bytes),
                disk_written: io.as_ref().map(|io| io.write_bytes),
                start_time: stat.starttime().get().ok().map(SystemTime::from),
                process_group_id: Some(stat.pgrp),
                session_id: Some(stat.session.into()),
                // The kernel's scheduling priority
                priority: Some(stat.priority),
                nice: Some(stat.nice),
                thread_count: Some(stat.num_threads),
                cwd: long
                    .then(|| proc.cwd().ok())
                    .flatten()
                    .map(|cwd| cwd.display().to_string()),
                environ: long.then(|| read_environ(&proc).ok()).flatten(),
            })
        })
        .collect()
}

/// Returns `comm`, or when the kernel cut it to its 15 byte limit, the file name of the first
/// path (argv[0], then the executable) whose name starts with it.
fn untruncated_name<'a>(comm: &str, paths: impl IntoIterator<Item = Option<&'a Path>>) -> String {
    let full_name = (comm.len() == COMM_MAX_LEN)
        .then(|| {
            paths
                .into_iter()
                .flatten()
                .filter_map(|path| path.file_name()?.to_str())
                .find(|name| name.starts_with(comm))
        })
        .flatten();
    full_name.unwrap_or(comm).to_string()
}

/// Names the state letter in `/proc/<pid>/stat`.
fn process_status(state: char) -> &'static str {
    match state {
        'S' => "Sleeping",
        'R' => "Running",
        'D' => "Disk sleep",
        'Z' => "Zombie",
        'T' => "Stopped",
        't' => "Tracing",
        'X' => "Dead",
        'x' => "Dead",
        'K' => "Wakekill",
        'W' => "Waking",
        'P' => "Parked",
        'I' => "Idle",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::untruncated_name;
    use std::path::Path;

    #[test]
    fn untruncated_name_restores_cut_names() {
        let argv0 = Path::new("/usr/libexec/gnome-shell-calendar-server");
        assert_eq!(
            untruncated_name("gnome-shell-cal", [Some(argv0), None]),
            "gnome-shell-calendar-server"
        );
        // argv[0] rewritten by the process: fall back to the executable.
        let exe = Path::new("/usr/lib/systemd/systemd-journald");
        assert_eq!(
            untruncated_name(
                "systemd-journal",
                [Some(Path::new("journald: main")), Some(exe)]
            ),
            "systemd-journald"
        );
    }

    #[test]
    fn untruncated_name_keeps_names_that_were_not_cut() {
        // Short names are never cut, even when argv[0] differs.
        assert_eq!(
            untruncated_name("bash", [Some(Path::new("-bash")), None]),
            "bash"
        );
        // A 15 byte name that no path extends stays as it is.
        assert_eq!(
            untruncated_name("abcdefghijklmno", [Some(Path::new("/bin/other")), None]),
            "abcdefghijklmno"
        );
    }
}
