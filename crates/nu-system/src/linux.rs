use log::info;
use procfs::process::{FDInfo, Io, Process, Stat, Status};
use procfs::{ProcError, ProcessCgroup};
use std::thread;
use std::time::{Duration, Instant};

pub enum ProcessTask {
    Process(Process),
    Task { stat: Box<Stat>, owner: u32 },
}

impl ProcessTask {
    pub fn stat(&self) -> Result<Stat, ProcError> {
        match self {
            ProcessTask::Process(x) => match x.stat() {
                Ok(it) => Ok(it),
                Err(err) => Err(err),
            },
            ProcessTask::Task { stat: x, owner: _ } => Ok(*x.clone()),
        }
    }

    pub fn cmdline(&self) -> Result<Vec<String>, ProcError> {
        match self {
            ProcessTask::Process(x) => x.cmdline(),
            _ => Err(ProcError::Other("not supported".to_string())),
        }
    }

    pub fn cgroups(&self) -> Result<Vec<ProcessCgroup>, ProcError> {
        match self {
            ProcessTask::Process(x) => x.cgroups(),
            _ => Err(ProcError::Other("not supported".to_string())),
        }
    }

    pub fn fd(&self) -> Result<Vec<FDInfo>, ProcError> {
        match self {
            ProcessTask::Process(x) => {
                let fds = match x.fd() {
                    Ok(f) => f,
                    Err(e) => return Err(e),
                };
                fds.collect()
            }
            _ => Err(ProcError::Other("not supported".to_string())),
        }
    }

    pub fn loginuid(&self) -> Result<u32, ProcError> {
        match self {
            ProcessTask::Process(x) => x.loginuid(),
            _ => Err(ProcError::Other("not supported".to_string())),
        }
    }

    pub fn owner(&self) -> u32 {
        match self {
            ProcessTask::Process(x) => x.uid().unwrap_or(0),
            ProcessTask::Task { stat: _, owner: x } => *x,
        }
    }

    pub fn wchan(&self) -> Result<String, ProcError> {
        match self {
            ProcessTask::Process(x) => x.wchan(),
            _ => Err(ProcError::Other("not supported".to_string())),
        }
    }
}

pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub curr_proc: ProcessTask,
    pub curr_io: Option<Io>,
    pub prev_io: Option<Io>,
    pub curr_stat: Option<Stat>,
    pub prev_stat: Option<Stat>,
    pub curr_status: Option<Status>,
    pub interval: Duration,
}

pub fn collect_proc(interval: Duration, _with_thread: bool) -> Vec<ProcessInfo> {
    let mut base_procs = Vec::new();
    let mut ret = Vec::new();

    // Take an initial snapshot of process I/O and CPU info, so we can calculate changes over time
    if let Ok(all_proc) = procfs::process::all_processes() {
        for proc in all_proc.flatten() {
            let io = proc.io().ok();
            let stat = proc.stat().ok();
            let time = Instant::now();
            base_procs.push((proc.pid(), io, stat, time));
        }
    }

    // wait a bit...
    thread::sleep(interval);

    // now get process info again, build up results
    for (pid, prev_io, prev_stat, prev_time) in base_procs {
        let curr_proc_pid = pid;
        let curr_proc = if let Ok(p) = Process::new(curr_proc_pid) {
            p
        } else {
            info!("failed to retrieve info for pid={curr_proc_pid}, process probably died between snapshots");
            continue;
        };

        let curr_io = curr_proc.io().ok();
        let curr_stat = curr_proc.stat().ok();
        let curr_status = curr_proc.status().ok();
        let curr_time = Instant::now();
        let interval = curr_time - prev_time;
        let ppid = match curr_proc.stat() {
            Ok(p) => p.ppid,
            Err(_) => 0,
        };
        let curr_proc = ProcessTask::Process(curr_proc);

        let proc = ProcessInfo {
            pid,
            ppid,
            curr_proc,
            curr_io,
            prev_io,
            curr_stat,
            prev_stat,
            curr_status,
            interval,
        };

        ret.push(proc);
    }

    ret
}

impl ProcessInfo {
    /// PID of process
    pub fn pid(&self) -> i32 {
        self.pid
    }

    /// Name of command
    pub fn name(&self) -> String {
        self.command()
            .split(' ')
            .collect::<Vec<_>>()
            .first()
            .map(|x| x.to_string())
            .unwrap_or_default()
    }

    /// Full name of command, with arguments
    pub fn command(&self) -> String {
        if let Ok(cmd) = &self.curr_proc.cmdline() {
            if !cmd.is_empty() {
                cmd.join(" ").replace(['\n', '\t'], " ")
            } else {
                match self.curr_proc.stat() {
                    Ok(p) => p.comm,
                    Err(_) => "".to_string(),
                }
            }
        } else {
            match self.curr_proc.stat() {
                Ok(p) => p.comm,
                Err(_) => "".to_string(),
            }
        }
    }

    /// Get the status of the process
    pub fn status(&self) -> String {
        match self.curr_proc.stat() {
            Ok(p) => match p.state {
                'S' => "Sleeping".into(),
                'R' => "Running".into(),
                'D' => "Disk sleep".into(),
                'Z' => "Zombie".into(),
                'T' => "Stopped".into(),
                't' => "Tracing".into(),
                'X' => "Dead".into(),
                'x' => "Dead".into(),
                'K' => "Wakekill".into(),
                'W' => "Waking".into(),
                'P' => "Parked".into(),
                _ => "Unknown".into(),
            },
            Err(_) => "Unknown".into(),
        }
    }

    /// CPU usage as a percent of total
    pub fn cpu_usage(&self) -> f64 {
        if let Some(cs) = &self.curr_stat {
            if let Some(ps) = &self.prev_stat {
                let curr_time = cs.utime + cs.stime;
                let prev_time = ps.utime + ps.stime;

                let usage_ms =
                    (curr_time - prev_time) * 1000 / procfs::ticks_per_second().unwrap_or(100);
                let interval_ms =
                    self.interval.as_secs() * 1000 + u64::from(self.interval.subsec_millis());
                usage_ms as f64 * 100.0 / interval_ms as f64
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Memory size in number of bytes
    pub fn mem_size(&self) -> u64 {
        match self.curr_proc.stat() {
            Ok(p) => p.rss_bytes().unwrap_or(0),
            Err(_) => 0,
        }
    }

    /// Virtual memory size in bytes
    pub fn virtual_size(&self) -> u64 {
        match self.curr_proc.stat() {
            Ok(p) => p.vsize,
            Err(_) => 0u64,
        }
    }
}
