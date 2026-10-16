use crate::process::{ProcessInfo, command_line};
use crate::unix::{UserNames, c_string};
use libc::{c_int, c_void, size_t};
use libproc::libproc::pid_rusage::{RUsageInfoV2, pidrusage};
use libproc::libproc::proc_pid::{ListThreads, listpidinfo, pidinfo, pidpath};
use libproc::libproc::task_info::TaskAllInfo;
use libproc::libproc::thread_info::ThreadInfo;
use libproc::processes::{ProcFilter, pids_by_type};
use mach2::mach_time;
use nix::unistd::{Pid, getsid};
use nu_utils::time::Instant;
use std::collections::HashMap;
use std::ffi::OsString;
use std::io;
use std::mem::{MaybeUninit, offset_of, size_of};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// One reading of the CPU time a process has used.
struct CpuSample {
    /// Start time in seconds and microseconds, to detect a reused pid.
    start: (u64, u64),
    /// User plus system time, in mach ticks.
    ticks: u64,
}

impl CpuSample {
    fn new(task: &TaskAllInfo) -> Self {
        Self {
            start: (task.pbsd.pbi_start_tvsec, task.pbsd.pbi_start_tvusec),
            ticks: task_cpu_ticks(task),
        }
    }
}

/// Lists every process, measuring CPU usage over `interval`. When `long` is false, the details
/// only `ps --long` shows aren't read, so they are `None`. The executable's path is still read
/// when the name needs it.
///
/// The kernel only hands out task, thread, argument and working directory details for the
/// current user's processes (`/bin/ps` and `top` are setuid root to see everything). The details
/// that need that access are `None` for other users' processes, which are still listed with the
/// details that are public.
pub fn collect_proc(interval: Duration, long: bool) -> Vec<ProcessInfo> {
    let mut base_procs = Vec::new();
    let arg_max = get_arg_max();

    if let Ok(procs) = pids_by_type(ProcFilter::All) {
        for p in procs {
            let sample = pidinfo::<TaskAllInfo>(p as i32, 0)
                .ok()
                .map(|task| CpuSample::new(&task));
            base_procs.push((p as i32, sample, Instant::now()));
        }
    }

    thread::sleep(interval);

    let ticktime = mach_ticktime();
    let kinfo = if long { all_kinfo() } else { HashMap::new() };
    let mut user_names = UserNames::default();
    base_procs
        .into_iter()
        .filter_map(|(pid, prev_sample, prev_time)| {
            // SAFETY: `proc_bsdshortinfo` is the struct this flavor returns, and it holds only
            // integers. A non-zero `arg` makes the kernel find zombies too, so this fails only
            // once the process has been reaped.
            let bsd_info: libc::proc_bsdshortinfo =
                unsafe { pid_info(pid, libc::PROC_PIDT_SHORTBSDINFO, 1) }?;
            let task = pidinfo::<TaskAllInfo>(pid, 0).ok();
            let interval = Instant::now().saturating_duration_since(prev_time);
            // A different start time means the pid was reused during the sample.
            let cpu_usage = prev_sample
                .zip(task.as_ref().map(CpuSample::new))
                .filter(|(prev, curr)| prev.start == curr.start)
                .map(|(prev, curr)| {
                    curr.ticks.saturating_sub(prev.ticks) as f64 * ticktime * 100.0
                        / interval.as_nanos() as f64
                });

            // Arguments, threads, resource usage and the working directory need the same access
            // as the task info, so don't ask for them when the kernel already refused.
            let (args, threads, rusage, cwd) = match &task {
                Some(task) => (
                    get_path_info(pid, arg_max),
                    get_threads(pid, task),
                    long.then(|| pidrusage::<RUsageInfoV2>(pid).ok()).flatten(),
                    long.then(|| get_cwd(pid)).flatten(),
                ),
                None => (None, Vec::new(), None, None),
            };
            // The name is the file name of the path the process was started with, and the
            // command its arguments. When those can't be read, both fall back to the executable,
            // like `/bin/ps` shows, and the name then to the kernel's (16 character) name.
            let started = args.as_ref().filter(|args| !args.cmd.is_empty());
            let exe = (long || started.is_none())
                .then(|| pidpath(pid).ok().map(PathBuf::from))
                .flatten();
            let name = started
                .map(|args| args.exe.as_path())
                .or(exe.as_deref())
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| {
                    c_string(&bsd_info.pbsi_comm.map(|c| c as u8)).unwrap_or_default()
                });
            let command = long
                .then(|| {
                    started
                        .and_then(|args| command_line(&args.cmd))
                        .or_else(|| exe.as_ref().map(|exe| exe.display().to_string()))
                })
                .flatten();
            let ptinfo = task.as_ref().map(|task| &task.ptinfo);
            let kinfo = kinfo.get(&pid);

            Some(ProcessInfo {
                pid,
                ppid: bsd_info.pbsi_ppid as i32,
                name,
                command,
                exe: exe.map(|exe| exe.display().to_string()),
                user: user_names.get(bsd_info.pbsi_uid),
                user_id: Some(bsd_info.pbsi_uid),
                status: process_status(bsd_info.pbsi_status, &threads),
                cpu_usage,
                cpu_time: task.as_ref().map(|task| {
                    Duration::from_nanos((task_cpu_ticks(task) as f64 * ticktime) as u64)
                }),
                mem_size: ptinfo.map(|info| info.pti_resident_size),
                virtual_size: ptinfo.map(|info| info.pti_virtual_size),
                // The physical footprint: the memory the process alone is responsible for,
                // including what the system compressed or swapped. Activity Monitor shows this
                // as "Memory".
                private_size: rusage.as_ref().map(|usage| usage.ri_phys_footprint),
                disk_read: rusage.as_ref().map(|usage| usage.ri_diskio_bytesread),
                disk_written: rusage.as_ref().map(|usage| usage.ri_diskio_byteswritten),
                start_time: kinfo.and_then(KinfoProc::start_time),
                process_group_id: Some(bsd_info.pbsi_pgid as i32),
                // `getsid(0)` reads the calling process, so kernel_task's session isn't readable.
                session_id: (long && pid != 0)
                    .then(|| getsid(Some(Pid::from_raw(pid))).ok())
                    .flatten()
                    .map(|sid| sid.as_raw().into()),
                // The Mach task priority
                priority: ptinfo.map(|info| info.pti_priority.into()),
                nice: kinfo.map(|kinfo| kinfo.p_nice.into()),
                thread_count: ptinfo.map(|info| info.pti_threadnum.into()),
                cwd: cwd.map(|cwd| cwd.display().to_string()),
                // The environment the process started with
                environ: args.filter(|_| long).map(|args| args.env),
            })
        })
        .collect()
}

/// Reads a fixed-size `proc_pidinfo` flavor, or `None` if the kernel refuses or the process
/// doesn't exist. `arg` means something different for each flavor.
///
/// # Safety
///
/// `T` must be the struct the kernel returns for `flavor`, and every bit pattern, including all
/// zeroes, must be a valid `T`: plain integers and arrays of them, no `bool`, enum or pointer.
unsafe fn pid_info<T>(pid: i32, flavor: c_int, arg: u64) -> Option<T> {
    let size = size_of::<T>() as c_int;
    let mut info = MaybeUninit::<T>::zeroed();
    // SAFETY: the buffer is `size` bytes, and the kernel writes at most that many.
    let written = unsafe { libc::proc_pidinfo(pid, flavor, arg, info.as_mut_ptr().cast(), size) };
    // SAFETY: the kernel filled the whole buffer, and the caller guarantees that any bytes make
    // a valid `T`.
    (written == size).then(|| unsafe { info.assume_init() })
}

fn get_threads(pid: i32, task: &TaskAllInfo) -> Vec<ThreadInfo> {
    listpidinfo::<ListThreads>(pid, task.ptinfo.pti_threadnum as usize)
        .map(|ids| {
            ids.into_iter()
                .filter_map(|id| pidinfo::<ThreadInfo>(pid, id).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// The real current working directory. The `PWD` environment variable only records the
/// directory the process was started in.
fn get_cwd(pid: i32) -> Option<PathBuf> {
    // SAFETY: `proc_vnodepathinfo` is the struct this flavor returns, and it holds only integers
    // and character arrays. The flavor ignores `arg`.
    let info: libc::proc_vnodepathinfo = unsafe { pid_info(pid, libc::PROC_PIDVNODEPATHINFO, 0) }?;
    let path: Vec<u8> = info
        .pvi_cdir
        .vip_path
        .as_flattened()
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    (!path.is_empty()).then(|| PathBuf::from(OsString::from_vec(path)))
}

/// Darwin's `struct kinfo_proc` from `<sys/sysctl.h>`, which starts with `struct extern_proc`
/// from `<sys/proc.h>`. libc doesn't define them for Apple targets, so this names only the
/// fields `ps` reads, at their offsets in the 648-byte struct of 64-bit targets, the size of each
/// entry `KERN_PROC_ALL` returns.
#[repr(C)]
struct KinfoProc {
    /// `kp_proc.p_starttime`
    p_starttime: libc::timeval,
    _before_pid: [u8; 40 - size_of::<libc::timeval>()],
    /// `kp_proc.p_pid`
    p_pid: libc::pid_t,
    _before_nice: [u8; 242 - 44],
    /// `kp_proc.p_nice`
    p_nice: libc::c_char,
    _rest: [u8; 648 - 243],
}

const _: () = assert!(
    offset_of!(KinfoProc, p_pid) == 40
        && offset_of!(KinfoProc, p_nice) == 242
        && size_of::<KinfoProc>() == 648
);

impl KinfoProc {
    /// When the process started
    fn start_time(&self) -> Option<SystemTime> {
        let start = self.p_starttime;
        let since_epoch = Duration::from_secs(start.tv_sec.try_into().ok()?)
            .checked_add(Duration::from_micros(start.tv_usec.try_into().ok()?))?;
        UNIX_EPOCH.checked_add(since_epoch)
    }
}

/// Reads every process's `kinfo_proc` with one `KERN_PROC_ALL` sysctl, keyed by pid. Unlike
/// `proc_pidinfo`, it covers every user's process, kernel_task and zombies included. Reading one
/// pid at a time with `KERN_PROC_PID` would walk the kernel's process list once per process.
fn all_kinfo() -> HashMap<i32, KinfoProc> {
    let mut mib = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_ALL];
    let mut sysctl = |buffer: *mut c_void, size: &mut size_t| {
        // SAFETY: `buffer` is either null, which only asks for the size, or has room for `size`
        // bytes, and the kernel writes at most that many.
        unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as u32,
                buffer,
                size,
                ptr::null_mut(),
                0,
            )
        }
    };
    // Processes can start between the size query and the read, which then fails with ENOMEM,
    // so leave some room and try again.
    for _ in 0..4 {
        let mut size = 0;
        if sysctl(ptr::null_mut(), &mut size) != 0 {
            break;
        }
        let mut procs = Vec::<KinfoProc>::with_capacity(size / size_of::<KinfoProc>() + 16);
        size = procs.capacity() * size_of::<KinfoProc>();
        if sysctl(procs.as_mut_ptr().cast(), &mut size) != 0 {
            if io::Error::last_os_error().raw_os_error() == Some(libc::ENOMEM) {
                continue;
            }
            break;
        }
        // SAFETY: the kernel wrote `size` bytes of whole structs, whose fields are all plain
        // integers.
        unsafe { procs.set_len(size / size_of::<KinfoProc>()) };
        return procs
            .into_iter()
            .map(|kinfo| (kinfo.p_pid, kinfo))
            .collect();
    }
    HashMap::new()
}

fn get_arg_max() -> size_t {
    let mut mib: [c_int; 2] = [libc::CTL_KERN, libc::KERN_ARGMAX];
    let mut arg_max = 0i32;
    let mut size = ::std::mem::size_of::<c_int>();
    unsafe {
        while libc::sysctl(
            mib.as_mut_ptr(),
            2,
            (&mut arg_max) as *mut i32 as *mut c_void,
            &mut size,
            ::std::ptr::null_mut(),
            0,
        ) == -1
        {}
    }
    arg_max as size_t
}

/// What `KERN_PROCARGS2` reports: the path passed to `exec`, the arguments and the environment
/// the process started with.
struct PathInfo {
    exe: PathBuf,
    cmd: Vec<String>,
    env: Vec<String>,
}

/// Decodes the bytes from `start` up to `cp`, replacing invalid UTF-8.
///
/// # Safety
///
/// `start..cp` must lie inside one initialized buffer.
unsafe fn lossy_str(cp: *mut u8, start: *mut u8) -> String {
    let len = (cp as usize).saturating_sub(start as usize);
    // SAFETY: the caller guarantees that `start..cp` is initialized memory in one buffer.
    let part = unsafe { std::slice::from_raw_parts(start, len) };
    String::from_utf8_lossy(part).into_owned()
}

fn get_path_info(pid: i32, mut size: size_t) -> Option<PathInfo> {
    let mut proc_args: Vec<u8> = Vec::with_capacity(size);
    let ptr: *mut u8 = proc_args.as_mut_ptr();

    let mut mib: [c_int; 3] = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as c_int];

    unsafe {
        let ret = libc::sysctl(
            mib.as_mut_ptr(),
            3,
            ptr as *mut c_void,
            &mut size,
            ::std::ptr::null_mut(),
            0,
        );
        if ret != -1 {
            let mut n_args: c_int = 0;
            libc::memcpy(
                (&mut n_args) as *mut c_int as *mut c_void,
                ptr as *const c_void,
                ::std::mem::size_of::<c_int>(),
            );
            let mut cp = ptr.add(::std::mem::size_of::<c_int>());
            let mut start = cp;
            if cp < ptr.add(size) {
                while cp < ptr.add(size) && *cp != 0 {
                    cp = cp.offset(1);
                }
                let exe = Path::new(lossy_str(cp, start).as_str()).to_path_buf();
                while cp < ptr.add(size) && *cp == 0 {
                    cp = cp.offset(1);
                }
                start = cp;
                let mut c = 0;
                let mut cmd = Vec::new();
                while c < n_args && cp < ptr.add(size) {
                    if *cp == 0 {
                        c += 1;
                        cmd.push(lossy_str(cp, start));
                        start = cp.offset(1);
                    }
                    cp = cp.offset(1);
                }
                start = cp;
                let mut env = Vec::new();
                while cp < ptr.add(size) {
                    if *cp == 0 {
                        if cp == start {
                            break;
                        }
                        env.push(lossy_str(cp, start));
                        start = cp.offset(1);
                    }
                    cp = cp.offset(1);
                }

                Some(PathInfo { exe, cmd, env })
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// The state of a process. Zombies and stopped processes report theirs in `pbsi_status`, and
/// every other process reports `SRUN` there, so its real state comes from its threads.
fn process_status(bsd_status: u32, threads: &[ThreadInfo]) -> Option<&'static str> {
    match bsd_status {
        libc::SZOMB => return Some("Zombie"),
        libc::SSTOP => return Some("Stopped"),
        _ => {}
    }
    let state = threads
        .iter()
        .map(|t| match t.pth_run_state {
            1 => 1, // TH_STATE_RUNNING
            2 => 5, // TH_STATE_STOPPED
            // The kernel reports a `pth_sleep_time` of 0 for every thread, so a long sleep
            // can't be told from a short one.
            3 => 3, // TH_STATE_WAITING
            4 => 2, // TH_STATE_UNINTERRUPTIBLE
            5 => 6, // TH_STATE_HALTED
            _ => 7,
        })
        .min()?;
    Some(match state {
        1 => "Running",
        2 => "Uninterruptible",
        3 => "Sleeping",
        5 => "Stopped",
        6 => "Halted",
        _ => "Unknown",
    })
}

/// User plus system time of a task, in mach ticks.
fn task_cpu_ticks(task: &TaskAllInfo) -> u64 {
    task.ptinfo.pti_total_user + task.ptinfo.pti_total_system
}

/// The Macos kernel returns process times in mach ticks rather than nanoseconds.  To get times in
/// nanoseconds, we need to multiply by the mach timebase, a fractional value reported by the
/// kernel.  It is uncertain if the kernel returns the same value on each call to
/// mach_timebase_info; if it does, it may be worth reimplementing this as a lazy_static value.
fn mach_ticktime() -> f64 {
    let mut timebase = mach_time::mach_timebase_info_data_t::default();
    let err = unsafe { mach_time::mach_timebase_info(&mut timebase) };
    if err == 0 {
        timebase.numer as f64 / timebase.denom as f64
    } else {
        // assume times are in nanoseconds then...
        1.0
    }
}
