use itertools::{EitherOrBoth, Itertools};
use libc::{
    CTL_HW, CTL_KERN, KERN_PROC, KERN_PROC_ARGS, KERN_PROC_CWD, KERN_PROC_ENV, KERN_PROC_PATHNAME,
    KERN_PROC_PROC, KERN_PROC_VMMAP, KVME_TYPE_DEFAULT, KVME_TYPE_SWAP, TDF_IDLETD, c_char,
    kinfo_file, kinfo_vmentry, sysctl,
};
use std::{
    io,
    mem::{self, MaybeUninit},
    ptr,
    time::{Duration, UNIX_EPOCH},
};

use crate::process::{ProcessInfo, name_and_command};
use crate::unix::{UserNames, c_string, split_nul};
use nu_utils::time::Instant;

/// Lists every process, measuring CPU usage over `interval`. When `long` is false, the details
/// only `ps --long` shows aren't read, so they are `None`.
///
/// The working directory and environment are only readable for the current user's processes (or
/// as root).
pub fn collect_proc(interval: Duration, long: bool) -> Vec<ProcessInfo> {
    compare_procs(interval, long).unwrap_or_else(|err| {
        log::warn!("Failed to get processes: {}", err);
        vec![]
    })
}

fn compare_procs(interval: Duration, long: bool) -> io::Result<Vec<ProcessInfo>> {
    let pagesize = get_pagesize()? as u64;

    // Compare two full snapshots of all of the processes over the interval
    let now = Instant::now();
    let procs_a = get_procs()?;
    std::thread::sleep(interval);
    let procs_b = get_procs()?;
    let true_interval = Instant::now().saturating_duration_since(now);
    let true_interval_sec = true_interval.as_secs_f64();

    let mut user_names = UserNames::default();

    // Join the processes between the two snapshots
    Ok(procs_a
        .into_iter()
        .merge_join_by(procs_b, |a, b| a.ki_pid.cmp(&b.ki_pid))
        .filter_map(|procs| {
            let (prev_proc, proc) = match procs {
                EitherOrBoth::Both(a, b) => (Some(a), b),
                // Started during the sample, so there's nothing to compare against.
                EitherOrBoth::Right(b) => (None, b),
                // Exited during the sample.
                EitherOrBoth::Left(_) => return None,
            };

            // Skip over the idle process. It always appears with high CPU usage when the
            // system is idle
            if proc.ki_tdflags as u64 & TDF_IDLETD as u64 != 0 {
                return None;
            }

            // The percentage CPU is the ratio of how much runtime occurred for the process out of
            // the true measured interval that occurred. `ki_runtime` is the process total in
            // microseconds, including threads that have exited. A different start time means
            // the pid was reused.
            let start = (proc.ki_start.tv_sec, proc.ki_start.tv_usec);
            let percent_cpu = prev_proc
                .filter(|prev| (prev.ki_start.tv_sec, prev.ki_start.tv_usec) == start)
                .map(|prev| {
                    let used = proc.ki_runtime.saturating_sub(prev.ki_runtime);
                    100. * used as f64 / 1_000_000. / true_interval_sec
                });

            #[allow(clippy::unnecessary_cast, reason = "`c_char` is `u8` on some targets")]
            let comm = proc.ki_comm.map(|c| c as u8);
            // Keep the process even when its arguments can't be read (a zombie, say).
            let (name, command) = name_and_command(
                &proc_sysctl(KERN_PROC_ARGS, proc.ki_pid).unwrap_or_default(),
                c_string(&comm).unwrap_or_default(),
            );

            Some(ProcessInfo {
                pid: proc.ki_pid,
                ppid: proc.ki_ppid,
                name,
                command: command.filter(|_| long),
                exe: long
                    .then(|| proc_sysctl(KERN_PROC_PATHNAME, proc.ki_pid).ok())
                    .flatten()
                    .and_then(|path| c_string(&path)),
                user: user_names.get(proc.ki_uid),
                user_id: Some(proc.ki_uid),
                status: Some(process_status(proc.ki_stat)),
                cpu_usage: percent_cpu,
                cpu_time: Some(Duration::from_micros(proc.ki_runtime)),
                mem_size: Some(proc.ki_rssize.max(0) as u64 * pagesize),
                virtual_size: Some(proc.ki_size as u64),
                private_size: long
                    .then(|| get_private_resident(proc.ki_pid, pagesize))
                    .flatten(),
                // `ki_rusage` counts storage I/O in blocks (`ru_inblock`, `ru_oublock`), not
                // bytes, and the kernel keeps no byte count to report.
                disk_read: None,
                disk_written: None,
                start_time: Some(
                    UNIX_EPOCH
                        + Duration::from_secs(proc.ki_start.tv_sec.max(0) as u64)
                        + Duration::from_micros(proc.ki_start.tv_usec.max(0) as u64),
                ),
                process_group_id: Some(proc.ki_pgid),
                session_id: Some(proc.ki_sid.into()),
                priority: Some(proc.ki_pri.pri_level.into()),
                nice: Some(proc.ki_nice.into()),
                thread_count: Some(proc.ki_numthreads.into()),
                cwd: long.then(|| get_cwd(proc.ki_pid)).flatten(),
                environ: long
                    .then(|| proc_sysctl(KERN_PROC_ENV, proc.ki_pid).ok())
                    .flatten()
                    .map(|env| split_nul(&env)),
            })
        })
        .collect())
}

fn check(err: libc::c_int) -> std::io::Result<()> {
    if err < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Lists every process. `KERN_PROC_PROC` gives one entry per process, whose `ki_runtime` is the
/// process's total, while `KERN_PROC_ALL` gives one per thread with that thread's own runtime.
fn get_procs() -> io::Result<Vec<libc::kinfo_proc>> {
    // To understand what's going on here, see the sysctl(3) manpage for FreeBSD.
    unsafe {
        const STRUCT_SIZE: usize = mem::size_of::<libc::kinfo_proc>();
        let ctl_name = [CTL_KERN, KERN_PROC, KERN_PROC_PROC];

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

        // Sort the procs by pid before using them
        vec.sort_by_key(|p| p.ki_pid);
        Ok(vec)
    }
}

/// Reads the variable-length `kern.proc.<what>.<pid>` sysctl, such as the arguments
/// (`KERN_PROC_ARGS`) or environment (`KERN_PROC_ENV`) of a process.
fn proc_sysctl(what: i32, pid: i32) -> io::Result<Vec<u8>> {
    unsafe {
        let ctl_name = [CTL_KERN, KERN_PROC, what, pid];

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

/// The working directory: the path in the `kinfo_file` that `KERN_PROC_CWD` returns.
fn get_cwd(pid: i32) -> Option<String> {
    let data = proc_sysctl(KERN_PROC_CWD, pid).ok()?;
    c_string(data.get(mem::offset_of!(kinfo_file, kf_path)..)?)
}

/// Resident memory of the process's anonymous mappings (heap, stack and anonymous `mmap`), the
/// resident pages of each `KERN_PROC_VMMAP` entry backed by anonymous memory, like Linux's
/// `RssAnon`. The entries' `kve_private_resident` (the `PRES` column of `procstat -v`) can't be
/// used: it is the page count of the whole VM object, so a mapped file such as a shared library
/// would add all of its cached pages, once for each of its mappings.
fn get_private_resident(pid: i32, pagesize: u64) -> Option<u64> {
    let data = proc_sysctl(KERN_PROC_VMMAP, pid).ok()?;
    let read_int = |offset: usize| {
        let bytes = data.get(offset..offset + mem::size_of::<i32>())?;
        Some(i32::from_ne_bytes(bytes.try_into().ok()?))
    };
    let mut pages = 0;
    let mut offset = 0;
    // The kernel packs the entries, so each one starts with its own size.
    while offset < data.len() {
        let size = read_int(offset + mem::offset_of!(kinfo_vmentry, kve_structsize))?;
        if size <= 0 {
            break;
        }
        let kind = read_int(offset + mem::offset_of!(kinfo_vmentry, kve_type))?;
        if kind == KVME_TYPE_DEFAULT || kind == KVME_TYPE_SWAP {
            let resident = read_int(offset + mem::offset_of!(kinfo_vmentry, kve_resident))?;
            pages += resident.max(0) as u64;
        }
        offset += size as usize;
    }
    Some(pages * pagesize)
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

/// Names a process state (`ki_stat`).
fn process_status(stat: c_char) -> &'static str {
    match stat {
        libc::SIDL | libc::SRUN => "Running",
        libc::SSLEEP => "Sleeping",
        libc::SSTOP => "Stopped",
        libc::SWAIT => "Waiting",
        libc::SLOCK => "Locked",
        libc::SZOMB => "Zombie",
        _ => "Unknown",
    }
}
