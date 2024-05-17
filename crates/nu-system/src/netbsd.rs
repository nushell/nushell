use libc::{sysctl, CTL_HW, CTL_KERN, KERN_PROC2, KERN_PROC_ALL, KERN_PROC_ARGS, KERN_PROC_ARGV};
use std::{
    io,
    mem::{self, MaybeUninit},
    ptr,
    time::{Duration, Instant},
};

pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub argv: Vec<u8>,
    pub stat: u64,
    pub lwp_stat: i8,
    pub percent_cpu: f64,
    pub mem_resident: u64, // in bytes
    pub mem_virtual: u64,  // in bytes
}

pub fn collect_proc(interval: Duration, _with_thread: bool) -> Vec<ProcessInfo> {
    compare_procs(interval).unwrap_or_else(|err| {
        log::warn!("Failed to get processes: {}", err);
        vec![]
    })
}

fn compare_procs(interval: Duration) -> io::Result<Vec<ProcessInfo>> {
    let pagesize = get_pagesize()? as u64;

    // Compare two full snapshots of all of the processes over the interval
    let now = Instant::now();
    let procs_a = get_procs()?;
    std::thread::sleep(interval);
    let true_interval = Instant::now().saturating_duration_since(now);
    let true_interval_sec = true_interval.as_secs_f64();
    let procs_b = get_procs()?;

    let mut a_iter = procs_a.into_iter().peekable();
    Ok(procs_b
        .into_iter()
        .map(|proc| {
            // Try to find the previous version of the process
            let mut prev_proc = None;
            while let Some(peek) = a_iter.peek() {
                if peek.p_pid < proc.p_pid {
                    continue;
                } else {
                    if peek.p_pid == proc.p_pid {
                        prev_proc = Some(a_iter.next().expect("a_iter.next() was None"));
                    }
                    break;
                }
            }

            // The percentage CPU is the ratio of how much runtime occurred for the process out of
            // the true measured interval that occurred.
            let percent_cpu = if let Some(prev_proc) = prev_proc {
                let prev_rtime =
                    prev_proc.p_rtime_sec as f64 + prev_proc.p_rtime_usec as f64 / 1_000_000.0;
                let rtime = proc.p_rtime_sec as f64 + proc.p_rtime_usec as f64 / 1_000_000.0;
                100. * (rtime - prev_rtime).max(0.) / true_interval_sec
            } else {
                0.0
            };

            Ok(ProcessInfo {
                pid: proc.p_pid,
                ppid: proc.p_ppid,
                argv: get_proc_args(proc.p_pid, KERN_PROC_ARGV)?,
                stat: proc.p_realstat,
                lwp_stat: proc.p_stat,
                percent_cpu,
                mem_resident: proc.p_vm_rssize.max(0) as u64 * pagesize,
                mem_virtual: proc.p_vm_msize.max(0) as u64 * pagesize,
            })
        })
        // Remove errors from the list - probably just processes that are gone now
        .flat_map(|result: io::Result<_>| result.ok())
        .collect())
}

fn check(err: libc::c_int) -> std::io::Result<()> {
    if err < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn get_procs() -> io::Result<Vec<libc::kinfo_proc2>> {
    // To understand what's going on here, see the sysctl(3) and sysctl(7) manpages for NetBSD.
    unsafe {
        const STRUCT_SIZE: usize = mem::size_of::<libc::kinfo_proc2>();
        let mut ctl_name = [
            CTL_KERN,
            KERN_PROC2,
            KERN_PROC_ALL,
            0,
            STRUCT_SIZE as i32,
            0,
        ];

        // First, try to figure out how large a buffer we need to allocate
        // (calling with NULL just tells us that)
        let mut data_len = 0;
        check(sysctl(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            ptr::null_mut(),
            &mut data_len,
            ptr::null(),
            0,
        ))?;

        // data_len will be set in bytes, so divide by the size of the structure
        let expected_len = data_len.div_ceil(STRUCT_SIZE);

        // Now allocate the Vec and set data_len to the real number of bytes allocated
        let mut vec: Vec<libc::kinfo_proc2> = Vec::with_capacity(expected_len);
        data_len = vec.capacity() * STRUCT_SIZE;

        // We are also supposed to set ctl_name[5] to the number of structures we want
        ctl_name[5] = expected_len.try_into().expect("expected_len too big");

        // Call sysctl() again to put the result in the vec
        check(sysctl(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            vec.as_mut_ptr() as *mut libc::c_void,
            &mut data_len,
            ptr::null(),
            0,
        ))?;

        // If that was ok, we can set the actual length of the vec to whatever
        // data_len was changed to, since that should now all be properly initialized data.
        let true_len = data_len.div_ceil(STRUCT_SIZE);
        vec.set_len(true_len);

        // Sort the procs by pid before using them
        vec.sort_by_key(|p| p.p_pid);
        Ok(vec)
    }
}

fn get_proc_args(pid: i32, what: i32) -> io::Result<Vec<u8>> {
    unsafe {
        let ctl_name = [CTL_KERN, KERN_PROC_ARGS, pid, what];

        // First, try to figure out how large a buffer we need to allocate
        // (calling with NULL just tells us that)
        let mut data_len = 0;
        check(sysctl(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            ptr::null_mut(),
            &mut data_len,
            ptr::null(),
            0,
        ))?;

        // Now allocate the Vec and set data_len to the real number of bytes allocated
        let mut vec: Vec<u8> = Vec::with_capacity(data_len);
        data_len = vec.capacity();

        // Call sysctl() again to put the result in the vec
        check(sysctl(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            vec.as_mut_ptr() as *mut libc::c_void,
            &mut data_len,
            ptr::null(),
            0,
        ))?;

        // If that was ok, we can set the actual length of the vec to whatever
        // data_len was changed to, since that should now all be properly initialized data.
        vec.set_len(data_len);
        Ok(vec)
    }
}

// For getting simple values from the sysctl interface
unsafe fn get_ctl<T>(ctl_name: &[i32]) -> io::Result<T> {
    let mut value: MaybeUninit<T> = MaybeUninit::uninit();
    let mut value_len = mem::size_of_val(&value);
    check(sysctl(
        ctl_name.as_ptr(),
        ctl_name.len() as u32,
        value.as_mut_ptr() as *mut libc::c_void,
        &mut value_len,
        ptr::null(),
        0,
    ))?;
    Ok(value.assume_init())
}

fn get_pagesize() -> io::Result<libc::c_int> {
    // not in libc for some reason
    const HW_PAGESIZE: i32 = 7;
    unsafe { get_ctl(&[CTL_HW, HW_PAGESIZE]) }
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
        self.argv
            .split(|b| *b == 0)
            .next()
            .map(String::from_utf8_lossy)
            .unwrap_or_default()
            .into_owned()
    }

    /// Full name of command, with arguments
    pub fn command(&self) -> String {
        if let Some(last_nul) = self.argv.iter().rposition(|b| *b == 0) {
            // The command string is NUL separated
            // Take the string up to the last NUL, then replace the NULs with spaces
            String::from_utf8_lossy(&self.argv[0..last_nul]).replace("\0", " ")
        } else {
            "".into()
        }
    }

    /// Get the status of the process
    pub fn status(&self) -> String {
        // see sys/proc.h, sys/lwp.h
        match self.stat {
            1 /* SIDL */ => "",
            2 /* SACTIVE */ => match self.lwp_stat {
                1 /* LSIDL */ => "",
                2 /* LSRUN */ => "Waiting",
                3 /* LSSLEEP */ => "Sleeping",
                4 /* LSSTOP */ => "Stopped",
                5 /* LSZOMB */ => "Zombie",
                7 /* LSONPROC */ => "Running",
                8 /* LSSUSPENDED */ => "Suspended",
                _ => "Unknown",
            },
            3 /* SDYING */ => "Dying",
            4 /* SSTOP */ => "Stopped",
            5 /* SZOMB */ => "Zombie",
            6 /* SDEAD */ => "Dead",
            _ => "Unknown",
        }
        .into()
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
