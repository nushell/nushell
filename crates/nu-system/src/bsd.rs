//! Used for FreeBSD, NetBSD, and OpenBSD.

use std::time::Duration;

use sysinfo::{ProcessesToUpdate, System};

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub name: String,
    pub argv: Vec<u8>,
    pub stat: String,
    pub percent_cpu: f64,
    pub mem_resident: u64, // in bytes
    pub mem_virtual: u64,  // in bytes
}

pub fn collect_proc(interval: Duration, _with_thread: bool) -> Vec<ProcessInfo> {
    let mut system = System::new();

    // First refresh — starts CPU tracking
    system.refresh_processes(ProcessesToUpdate::All, false);

    std::thread::sleep(interval);

    // Second refresh — cpu_usage() is now computed over the interval
    system.refresh_processes(ProcessesToUpdate::All, false);

    system
        .processes()
        .iter()
        .map(|(&pid, process)| {
            let ppid = process.parent().map(|p| p.as_u32() as i32).unwrap_or(0);
            let name = process.name().to_string_lossy().to_string();
            let argv: Vec<u8> = process
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("\0")
                .into_bytes();
            let stat = format!("{:?}", process.status());
            let percent_cpu = process.cpu_usage() as f64;
            let mem_resident = process.memory();
            let mem_virtual = process.virtual_memory();

            ProcessInfo {
                pid: pid.as_u32() as i32,
                ppid,
                name,
                argv,
                stat,
                percent_cpu,
                mem_resident,
                mem_virtual,
            }
        })
        .collect()
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
        let argv_name = self
            .argv
            .split(|b| *b == 0)
            .next()
            .map(String::from_utf8_lossy)
            .unwrap_or_default()
            .into_owned();

        if !argv_name.is_empty() {
            argv_name
        } else {
            self.name.clone()
        }
    }

    /// Full name of command, with arguments
    pub fn command(&self) -> String {
        if let Some(last_nul) = self.argv.iter().rposition(|b| *b == 0) {
            // The command string is NUL-separated; replace NULs with spaces
            String::from_utf8_lossy(&self.argv[0..last_nul]).replace("\0", " ")
        } else {
            self.name()
        }
    }

    /// Get the status of the process
    pub fn status(&self) -> String {
        self.stat.clone()
    }

    /// CPU usage as a percent of total
    pub fn cpu_usage(&self) -> f64 {
        self.percent_cpu
    }

    /// Memory size in number of bytes
    pub fn mem_size(&self) -> u64 {
        self.mem_resident
    }

    /// Virtual memory size in bytes
    pub fn virtual_size(&self) -> u64 {
        self.mem_virtual
    }
}
