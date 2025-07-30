use itertools::{EitherOrBoth, Itertools};
use libc::{
    CTL_HW, CTL_KERN, KERN_PROC, KERN_PROC_ALL, KERN_PROC_ARGS, TDF_IDLETD, c_char, kinfo_proc,
    sysctl,
};
use std::{
    ffi::CStr,
    io,
    mem::{self, MaybeUninit},
    ptr,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub name: String,
    pub argv: Vec<u8>,
    pub stat: c_char,
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
    let procs_b = get_procs()?;
    let true_interval = Instant::now().saturating_duration_since(now);
    let true_interval_sec = true_interval.as_secs_f64();

    // Group all of the threads in each process together
    let a_grouped = procs_a.into_iter().group_by(|proc| proc.ki_pid);
    let b_grouped = procs_b.into_iter().group_by(|proc| proc.ki_pid);

    // Join the processes between the two snapshots
    Ok(a_grouped
        .into_iter()
        .merge_join_by(b_grouped.into_iter(), |(pid_a, _), (pid_b, _)| {
            pid_a.cmp(pid_b)
        })
        .map(|threads| {
            // Join the threads between the two snapshots for the process
            let mut threads = {
                let (left, right) = threads.left_and_right();
                left.into_iter()
                    .flat_map(|(_, threads)| threads)
                    .merge_join_by(
                        right.into_iter().flat_map(|(_, threads)| threads),
                        |thread_a, thread_b| thread_a.ki_tid.cmp(&thread_b.ki_tid),
                    )
                    .peekable()
            };

            // Pick the later process entry of the first thread to use for basic process information
            let proc = match threads.peek().ok_or(io::ErrorKind::NotFound)? {
                EitherOrBoth::Both(_, b) => b,
                EitherOrBoth::Left(a) => a,
                EitherOrBoth::Right(b) => b,
            }
            .clone();

            // Skip over the idle process. It always appears with high CPU usage when the
            // system is idle
            if proc.ki_tdflags as u64 & TDF_IDLETD as u64 != 0 {
                return Err(io::ErrorKind::NotFound.into());
            }

            // Aggregate all of the threads that exist in both snapshots and sum their runtime.
            let (runtime_a, runtime_b) =
                threads
                    .flat_map(|t| t.both())
                    .fold((0., 0.), |(runtime_a, runtime_b), (a, b)| {
                        let runtime_in_seconds =
                            |proc: &kinfo_proc| proc.ki_runtime as f64 /* µsec */ / 1_000_000.0;
                        (
                            runtime_a + runtime_in_seconds(&a),
                            runtime_b + runtime_in_seconds(&b),
                        )
                    });

            // The percentage CPU is the ratio of how much runtime occurred for the process out of
            // the true measured interval that occurred.
            let percent_cpu = 100. * (runtime_b - runtime_a).max(0.) / true_interval_sec;

            let info = ProcessInfo {
                pid: proc.ki_pid,
                ppid: proc.ki_ppid,
                name: read_cstr(&proc.ki_comm).to_string_lossy().into_owned(),
                argv: get_proc_args(proc.ki_pid)?,
                stat: proc.ki_stat,
                percent_cpu,
                mem_resident: proc.ki_rssize.max(0) as u64 * pagesize,
                mem_virtual: proc.ki_size.max(0) as u64,
            };
            Ok(info)
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

/// This is a bounds-checked way to read a `CStr` from a slice of `c_char`
fn read_cstr(slice: &[libc::c_char]) -> &CStr {
    unsafe {
        // SAFETY: ensure that c_char and u8 are the same size
        mem::transmute::<libc::c_char, u8>(0);
        let slice: &[u8] = mem::transmute(slice);
        CStr::from_bytes_until_nul(slice).unwrap_or_default()
    }
}

fn get_procs() -> io::Result<Vec<libc::kinfo_proc>> {
    // To understand what's going on here, see the sysctl(3) manpage for FreeBSD.
    unsafe {
        const STRUCT_SIZE: usize = mem::size_of::<libc::kinfo_proc>();
        let ctl_name = [CTL_KERN, KERN_PROC, KERN_PROC_ALL];

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
        let mut vec: Vec<libc::kinfo_proc> = Vec::with_capacity(expected_len);
        data_len = vec.capacity() * STRUCT_SIZE;

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

        // Sort the procs by pid and then tid before using them
        vec.sort_by_key(|p| (p.ki_pid, p.ki_tid));
        Ok(vec)
    }
}

fn get_proc_args(pid: i32) -> io::Result<Vec<u8>> {
    unsafe {
        let ctl_name = [CTL_KERN, KERN_PROC, KERN_PROC_ARGS, pid];

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

/// For getting simple values from the sysctl interface
///
/// # Safety
/// `T` needs to be of the structure that is expected to be returned by `sysctl` for the given
/// `ctl_name` sequence and will then be assumed to be of correct layout.
/// Thus only use it for primitive types or well defined fixed size types. For variable length
/// arrays that can be returned from `sysctl` use it directly (or write a proper wrapper handling
/// capacity management)
///
/// # Panics
/// If the size of the returned data diverges from the size of the expected `T`
unsafe fn get_ctl<T>(ctl_name: &[i32]) -> io::Result<T> {
    let mut value: MaybeUninit<T> = MaybeUninit::uninit();
    let mut value_len = mem::size_of_val(&value);
    // SAFETY: lengths to the pointers is provided, uninitialized data with checked length provided
    // Only assume initialized when the written data doesn't diverge in length, layout is the
    // safety responsibility of the caller.
    check(unsafe {
        sysctl(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            value.as_mut_ptr() as *mut libc::c_void,
            &mut value_len,
            ptr::null(),
            0,
        )
    })?;
    assert_eq!(
        value_len,
        mem::size_of_val(&value),
        "Data requested from from `sysctl` diverged in size from the expected return type. For variable length data you need to manually truncate the data to the valid returned size!"
    );
    Ok(unsafe { value.assume_init() })
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
            // Just use the command name alone.
            self.name.clone()
        }
    }

    /// Full name of command, with arguments
    pub fn command(&self) -> String {
        if let Some(last_nul) = self.argv.iter().rposition(|b| *b == 0) {
            // The command string is NUL separated
            // Take the string up to the last NUL, then replace the NULs with spaces
            String::from_utf8_lossy(&self.argv[0..last_nul]).replace("\0", " ")
        } else {
            // The argv is empty, so use the name instead
            self.name()
        }
    }

    /// Get the status of the process
    pub fn status(&self) -> String {
        match self.stat {
            libc::SIDL | libc::SRUN => "Running",
            libc::SSLEEP => "Sleeping",
            libc::SSTOP => "Stopped",
            libc::SWAIT => "Waiting",
            libc::SLOCK => "Locked",
            libc::SZOMB => "Zombie",
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
