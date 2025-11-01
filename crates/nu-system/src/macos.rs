use libc::{c_int, c_void, size_t};
use libproc::libproc::bsd_info::BSDInfo;
use libproc::libproc::file_info::{ListFDs, ProcFDType, pidfdinfo};
use libproc::libproc::net_info::{InSockInfo, SocketFDInfo, SocketInfoKind, TcpSockInfo};
use libproc::libproc::pid_rusage::{RUsageInfoV2, pidrusage};
use libproc::libproc::proc_pid::{ListThreads, listpidinfo, pidinfo};
use libproc::libproc::task_info::{TaskAllInfo, TaskInfo};
use libproc::libproc::thread_info::ThreadInfo;
use libproc::processes::{ProcFilter, pids_by_type};
use mach2::mach_time;
use std::cmp;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub curr_task: TaskAllInfo,
    pub prev_task: TaskAllInfo,
    pub curr_path: Option<PathInfo>,
    pub curr_threads: Vec<ThreadInfo>,
    pub curr_udps: Vec<InSockInfo>,
    pub curr_tcps: Vec<TcpSockInfo>,
    pub curr_res: Option<RUsageInfoV2>,
    pub prev_res: Option<RUsageInfoV2>,
    pub interval: Duration,
    pub start_time: i64,
    pub user_id: i64,
    pub priority: i64,
    pub task_thread_num: i64,
}

pub fn collect_proc(interval: Duration, _with_thread: bool) -> Vec<ProcessInfo> {
    let mut base_procs = Vec::new();
    let mut ret = Vec::new();
    let arg_max = get_arg_max();

    if let Ok(procs) = pids_by_type(ProcFilter::All) {
        for p in procs {
            if let Ok(task) = pidinfo::<TaskAllInfo>(p as i32, 0) {
                let res = pidrusage::<RUsageInfoV2>(p as i32).ok();
                let time = Instant::now();
                base_procs.push((p as i32, task, res, time));
            }
        }
    }

    thread::sleep(interval);

    for (pid, prev_task, prev_res, prev_time) in base_procs {
        let curr_task = if let Ok(task) = pidinfo::<TaskAllInfo>(pid, 0) {
            task
        } else {
            clone_task_all_info(&prev_task)
        };

        let curr_path = get_path_info(pid, arg_max);

        let threadids = listpidinfo::<ListThreads>(pid, curr_task.ptinfo.pti_threadnum as usize);
        let mut curr_threads = Vec::new();
        if let Ok(threadids) = threadids {
            for t in threadids {
                if let Ok(thread) = pidinfo::<ThreadInfo>(pid, t) {
                    curr_threads.push(thread);
                }
            }
        }

        let mut curr_tcps = Vec::new();
        let mut curr_udps = Vec::new();

        let fds = listpidinfo::<ListFDs>(pid, curr_task.pbsd.pbi_nfiles as usize);
        if let Ok(fds) = fds {
            for fd in fds {
                if let ProcFDType::Socket = fd.proc_fdtype.into()
                    && let Ok(socket) = pidfdinfo::<SocketFDInfo>(pid, fd.proc_fd)
                {
                    match socket.psi.soi_kind.into() {
                        SocketInfoKind::In => {
                            if socket.psi.soi_protocol == libc::IPPROTO_UDP {
                                let info = unsafe { socket.psi.soi_proto.pri_in };
                                curr_udps.push(info);
                            }
                        }
                        SocketInfoKind::Tcp => {
                            let info = unsafe { socket.psi.soi_proto.pri_tcp };
                            curr_tcps.push(info);
                        }
                        _ => (),
                    }
                }
            }
        }

        let curr_res = pidrusage::<RUsageInfoV2>(pid).ok();

        let curr_time = Instant::now();
        let interval = curr_time.saturating_duration_since(prev_time);
        let ppid = curr_task.pbsd.pbi_ppid as i32;
        let start_time = curr_task.pbsd.pbi_start_tvsec as i64;
        let user_id = curr_task.pbsd.pbi_uid as i64;
        let priority = curr_task.ptinfo.pti_priority as i64;
        let task_thread_num = curr_task.ptinfo.pti_threadnum as i64;

        let proc = ProcessInfo {
            pid,
            ppid,
            curr_task,
            prev_task,
            curr_path,
            curr_threads,
            curr_udps,
            curr_tcps,
            curr_res,
            prev_res,
            interval,
            start_time,
            user_id,
            priority,
            task_thread_num,
        };

        ret.push(proc);
    }

    ret
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

pub struct PathInfo {
    pub name: String,
    pub exe: PathBuf,
    pub root: PathBuf,
    pub cmd: Vec<String>,
    pub env: Vec<String>,
    pub cwd: PathBuf,
}

unsafe fn get_unchecked_str(cp: *mut u8, start: *mut u8) -> String {
    unsafe {
        let len = (cp as usize).saturating_sub(start as usize);
        let part = std::slice::from_raw_parts(start, len);
        String::from_utf8_unchecked(part.to_vec())
    }
}

fn get_path_info(pid: i32, mut size: size_t) -> Option<PathInfo> {
    let mut proc_args = Vec::with_capacity(size);
    let ptr: *mut u8 = proc_args.as_mut_slice().as_mut_ptr();

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
                let exe = Path::new(get_unchecked_str(cp, start).as_str()).to_path_buf();
                let name = exe
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("")
                    .to_owned();
                let mut need_root = true;
                let mut root = Default::default();
                if exe.is_absolute()
                    && let Some(parent) = exe.parent()
                {
                    root = parent.to_path_buf();
                    need_root = false;
                }
                while cp < ptr.add(size) && *cp == 0 {
                    cp = cp.offset(1);
                }
                start = cp;
                let mut c = 0;
                let mut cmd = Vec::new();
                while c < n_args && cp < ptr.add(size) {
                    if *cp == 0 {
                        c += 1;
                        cmd.push(get_unchecked_str(cp, start));
                        start = cp.offset(1);
                    }
                    cp = cp.offset(1);
                }
                start = cp;
                let mut env = Vec::new();
                let mut cwd = PathBuf::default();
                while cp < ptr.add(size) {
                    if *cp == 0 {
                        if cp == start {
                            break;
                        }
                        let env_str = get_unchecked_str(cp, start);
                        if let Some(pwd) = env_str.strip_prefix("PWD=") {
                            cwd = PathBuf::from(pwd)
                        }
                        env.push(env_str);
                        start = cp.offset(1);
                    }
                    cp = cp.offset(1);
                }
                if need_root {
                    for env in env.iter() {
                        if env.starts_with("PATH=") {
                            root = Path::new(&env[6..]).to_path_buf();
                            break;
                        }
                    }
                }

                Some(PathInfo {
                    exe,
                    name,
                    root,
                    cmd,
                    env,
                    cwd,
                })
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn clone_task_all_info(src: &TaskAllInfo) -> TaskAllInfo {
    let pbsd = BSDInfo {
        pbi_flags: src.pbsd.pbi_flags,
        pbi_status: src.pbsd.pbi_status,
        pbi_xstatus: src.pbsd.pbi_xstatus,
        pbi_pid: src.pbsd.pbi_pid,
        pbi_ppid: src.pbsd.pbi_ppid,
        pbi_uid: src.pbsd.pbi_uid,
        pbi_gid: src.pbsd.pbi_gid,
        pbi_ruid: src.pbsd.pbi_ruid,
        pbi_rgid: src.pbsd.pbi_rgid,
        pbi_svuid: src.pbsd.pbi_svuid,
        pbi_svgid: src.pbsd.pbi_svgid,
        rfu_1: src.pbsd.rfu_1,
        pbi_comm: src.pbsd.pbi_comm,
        pbi_name: src.pbsd.pbi_name,
        pbi_nfiles: src.pbsd.pbi_nfiles,
        pbi_pgid: src.pbsd.pbi_pgid,
        pbi_pjobc: src.pbsd.pbi_pjobc,
        e_tdev: src.pbsd.e_tdev,
        e_tpgid: src.pbsd.e_tpgid,
        pbi_nice: src.pbsd.pbi_nice,
        pbi_start_tvsec: src.pbsd.pbi_start_tvsec,
        pbi_start_tvusec: src.pbsd.pbi_start_tvusec,
    };

    // Comments taken from here https://github.com/apple-oss-distributions/xnu/blob/8d741a5de7ff4191bf97d57b9f54c2f6d4a15585/bsd/sys/proc_info.h#L127
    let ptinfo = TaskInfo {
        // virtual memory size (bytes)
        pti_virtual_size: src.ptinfo.pti_virtual_size,
        // resident memory size (bytes)
        pti_resident_size: src.ptinfo.pti_resident_size,
        // total user time
        pti_total_user: src.ptinfo.pti_total_user,
        // total system time
        pti_total_system: src.ptinfo.pti_total_system,
        // existing threads only user
        pti_threads_user: src.ptinfo.pti_threads_user,
        // existing threads only system
        pti_threads_system: src.ptinfo.pti_threads_system,
        // default policy for new threads
        pti_policy: src.ptinfo.pti_policy,
        // number of page faults
        pti_faults: src.ptinfo.pti_faults,
        // number of actual pageins
        pti_pageins: src.ptinfo.pti_pageins,
        // number of copy-on-write faults
        pti_cow_faults: src.ptinfo.pti_cow_faults,
        // number of messages sent
        pti_messages_sent: src.ptinfo.pti_messages_sent,
        // number of messages received
        pti_messages_received: src.ptinfo.pti_messages_received,
        // number of mach system calls
        pti_syscalls_mach: src.ptinfo.pti_syscalls_mach,
        // number of unix system calls
        pti_syscalls_unix: src.ptinfo.pti_syscalls_unix,
        // number of context switches
        pti_csw: src.ptinfo.pti_csw,
        // number of threads in the task
        pti_threadnum: src.ptinfo.pti_threadnum,
        // number of running threads
        pti_numrunning: src.ptinfo.pti_numrunning,
        // task priority
        pti_priority: src.ptinfo.pti_priority,
    };
    TaskAllInfo { pbsd, ptinfo }
}

impl ProcessInfo {
    /// PID of process
    pub fn pid(&self) -> i32 {
        self.pid
    }

    /// Parent PID of process
    pub fn ppid(&self) -> i32 {
        self.ppid
    }

    /// Name of command
    pub fn name(&self) -> String {
        if let Some(path) = &self.curr_path {
            if !path.cmd.is_empty() {
                let command_path = &path.exe;

                if let Some(command_name) = command_path.file_name() {
                    command_name.to_string_lossy().to_string()
                } else {
                    command_path.to_string_lossy().to_string()
                }
            } else {
                String::from("")
            }
        } else {
            String::from("")
        }
    }

    /// Full name of command, with arguments
    pub fn command(&self) -> String {
        if let Some(path) = &self.curr_path {
            if !path.cmd.is_empty() {
                path.cmd.join(" ").replace(['\n', '\t'], " ")
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    /// Get the status of the process
    pub fn status(&self) -> String {
        let mut state = 7;
        for t in &self.curr_threads {
            let s = match t.pth_run_state {
                1 => 1, // TH_STATE_RUNNING
                2 => 5, // TH_STATE_STOPPED
                3 => {
                    if t.pth_sleep_time > 20 {
                        4
                    } else {
                        3
                    }
                } // TH_STATE_WAITING
                4 => 2, // TH_STATE_UNINTERRUPTIBLE
                5 => 6, // TH_STATE_HALTED
                _ => 7,
            };
            state = cmp::min(s, state);
        }
        let state = match state {
            0 => "",
            1 => "Running",
            2 => "Uninterruptible",
            3 => "Sleep",
            4 => "Waiting",
            5 => "Stopped",
            6 => "Halted",
            _ => "?",
        };
        state.to_string()
    }

    /// CPU usage as a percent of total
    pub fn cpu_usage(&self) -> f64 {
        let curr_time =
            self.curr_task.ptinfo.pti_total_user + self.curr_task.ptinfo.pti_total_system;
        let prev_time =
            self.prev_task.ptinfo.pti_total_user + self.prev_task.ptinfo.pti_total_system;
        let usage_ticks = curr_time.saturating_sub(prev_time);
        let interval_us = self.interval.as_micros();
        let ticktime_us = mach_ticktime() / 1000.0;
        usage_ticks as f64 * 100.0 * ticktime_us / interval_us as f64
    }

    /// Memory size in number of bytes
    pub fn mem_size(&self) -> u64 {
        self.curr_task.ptinfo.pti_resident_size
    }

    /// Virtual memory size in bytes
    pub fn virtual_size(&self) -> u64 {
        self.curr_task.ptinfo.pti_virtual_size
    }

    pub fn cwd(&self) -> String {
        self.curr_path
            .as_ref()
            .map(|cur_path| cur_path.cwd.display().to_string())
            .unwrap_or_default()
    }
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
