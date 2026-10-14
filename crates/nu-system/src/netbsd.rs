//! This is used for both NetBSD and OpenBSD, because they are fairly similar.

use itertools::{EitherOrBoth, Itertools};
use libc::{
    CTL_HW, CTL_KERN, KERN_PROC_ALL, KERN_PROC_ARGS, KERN_PROC_ARGV, KERN_PROC_ENV, sysctl,
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

#[cfg(target_os = "netbsd")]
type KInfoProc = libc::kinfo_proc2;
#[cfg(target_os = "openbsd")]
type KInfoProc = libc::kinfo_proc;

/// The kernel stores `p_nice` offset by `NZERO` (from `<sys/param.h>`), so that it fits in an
/// unsigned byte. libc only defines `NZERO` for FreeBSD, where it is 0.
const NZERO: i64 = 20;

/// The `kern.proc_args.<pid>.cwd` sysctl from NetBSD's `<sys/sysctl.h>`, which libc doesn't
/// define.
#[cfg(target_os = "netbsd")]
const KERN_PROC_CWD: i32 = 6;

/// Lists every process, measuring CPU usage over `interval`. When `long` is false, the details
/// only `ps --long` shows aren't read, so they are `None`.
///
/// The environment is only readable for the current user's processes (or as root).
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
        .merge_join_by(procs_b, |a, b| a.p_pid.cmp(&b.p_pid))
        .filter_map(|procs| {
            let (prev_proc, proc) = match procs {
                EitherOrBoth::Both(a, b) => (Some(a), b),
                // Started during the sample, so there's nothing to compare against.
                EitherOrBoth::Right(b) => (None, b),
                // Exited during the sample.
                EitherOrBoth::Left(_) => return None,
            };

            // The kernel only reports the run and start times (`p_u*`) of a process that isn't
            // a zombie (`p_uvalid`), and OpenBSD leaves the start time out for one that is exiting.
            let rtime =
                |proc: &KInfoProc| proc.p_rtime_sec as f64 + proc.p_rtime_usec as f64 / 1_000_000.0;
            let start_time = (proc.p_uvalid != 0 && proc.p_ustart_sec != 0).then(|| {
                UNIX_EPOCH
                    + Duration::from_secs(proc.p_ustart_sec as u64)
                    + Duration::from_micros(proc.p_ustart_usec as u64)
            });

            // The percentage CPU is the ratio of how much runtime occurred for the process out of
            // the true measured interval that occurred. A different start time means the pid was
            // reused.
            let same_process = |prev: &KInfoProc| {
                start_time.is_none()
                    || (prev.p_ustart_sec, prev.p_ustart_usec)
                        == (proc.p_ustart_sec, proc.p_ustart_usec)
            };
            let percent_cpu = prev_proc.filter(same_process).map(|prev_proc| {
                100. * (rtime(&proc) - rtime(&prev_proc)).max(0.) / true_interval_sec
            });

            #[allow(clippy::unnecessary_cast, reason = "`c_char` is `u8` on some targets")]
            let comm = proc.p_comm.map(|c| c as u8);
            // Keep the process even when its arguments can't be read (a zombie, say).
            let (name, command) = name_and_command(
                &get_proc_args(proc.p_pid, KERN_PROC_ARGV).unwrap_or_default(),
                c_string(&comm).unwrap_or_default(),
            );

            Some(ProcessInfo {
                pid: proc.p_pid,
                ppid: proc.p_ppid,
                name,
                command: command.filter(|_| long),
                #[cfg(target_os = "netbsd")]
                exe: long
                    .then(|| get_proc_args(proc.p_pid, libc::KERN_PROC_PATHNAME).ok())
                    .flatten()
                    .and_then(|path| c_string(&path)),
                // OpenBSD doesn't expose the path of a process's executable.
                #[cfg(target_os = "openbsd")]
                exe: None,
                user: user_names.get(proc.p_uid),
                user_id: Some(proc.p_uid),
                status: Some(process_status(proc.p_stat)),
                cpu_usage: percent_cpu,
                cpu_time: (proc.p_uvalid != 0).then(|| Duration::from_secs_f64(rtime(&proc))),
                mem_size: Some(proc.p_vm_rssize.max(0) as u64 * pagesize),
                #[cfg(target_os = "netbsd")]
                virtual_size: Some(proc.p_vm_msize.max(0) as u64 * pagesize),
                // OpenBSD never fills in `p_vm_map_size`, so add up the text, data and stack
                // segments, as its `ps` and `top` do.
                #[cfg(target_os = "openbsd")]
                virtual_size: Some(
                    (proc.p_vm_tsize.max(0) as u64
                        + proc.p_vm_dsize.max(0) as u64
                        + proc.p_vm_ssize.max(0) as u64)
                        * pagesize,
                ),
                // The kernel only reports a process's total resident size (`p_vm_rssize`) and
                // the virtual sizes of its segments, nothing about what is private.
                private_size: None,
                // `kinfo_proc` counts storage I/O in blocks (`p_uru_inblock`, `p_uru_oublock`),
                // not bytes, and the kernel keeps no byte count to report.
                disk_read: None,
                disk_written: None,
                start_time,
                process_group_id: Some(proc.p__pgid),
                session_id: Some(proc.p_sid.into()),
                priority: Some(proc.p_priority as i64),
                nice: Some(i64::from(proc.p_nice) - NZERO),
                #[cfg(target_os = "netbsd")]
                thread_count: Some(proc.p_nlwps as i64),
                // OpenBSD's `kinfo_proc` has no thread count; counting threads means listing
                // them with `KERN_PROC_SHOW_THREADS`.
                #[cfg(target_os = "openbsd")]
                thread_count: None,
                cwd: long.then(|| get_cwd(proc.p_pid)).flatten(),
                environ: long
                    .then(|| get_proc_args(proc.p_pid, KERN_PROC_ENV).ok())
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

/// Call `sysctl()` in read mode (i.e. the last two arguments to set new values are NULL and zero)
///
/// `name` is a flag array.
///
/// # Safety
/// `data` needs to be writable for `data_len` or be a `ptr::null()` paired with `data_len = 0` to
/// poll for the expected length in the `data_len` out parameter.
///
/// For more details see: https://man.netbsd.org/sysctl.3
unsafe fn sysctl_get(
    name: *const i32,
    name_len: u32,
    data: *mut libc::c_void,
    data_len: *mut usize,
) -> i32 {
    // Safety: Call to unsafe function `libc::sysctl`
    unsafe {
        sysctl(
            name,
            name_len,
            data,
            data_len,
            // NetBSD and OpenBSD differ in mutability for this pointer, but it's null anyway
            #[cfg(target_os = "netbsd")]
            ptr::null(),
            #[cfg(target_os = "openbsd")]
            ptr::null_mut(),
            0,
        )
    }
}

fn get_procs() -> io::Result<Vec<KInfoProc>> {
    // To understand what's going on here, see the sysctl(3) and sysctl(7) manpages for NetBSD.
    unsafe {
        const STRUCT_SIZE: usize = mem::size_of::<KInfoProc>();

        #[cfg(target_os = "netbsd")]
        const TGT_KERN_PROC: i32 = libc::KERN_PROC2;
        #[cfg(target_os = "openbsd")]
        const TGT_KERN_PROC: i32 = libc::KERN_PROC;

        let mut ctl_name = [
            CTL_KERN,
            TGT_KERN_PROC,
            KERN_PROC_ALL,
            0,
            STRUCT_SIZE as i32,
            0,
        ];

        // First, try to figure out how large a buffer we need to allocate
        // (calling with NULL just tells us that)
        let mut data_len = 0;
        check(sysctl_get(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            ptr::null_mut(),
            &mut data_len,
        ))?;

        // data_len will be set in bytes, so divide by the size of the structure
        let expected_len = data_len.div_ceil(STRUCT_SIZE);

        // Now allocate the Vec and set data_len to the real number of bytes allocated
        let mut vec: Vec<KInfoProc> = Vec::with_capacity(expected_len);
        data_len = vec.capacity() * STRUCT_SIZE;

        // We are also supposed to set ctl_name[5] to the number of structures we want
        ctl_name[5] = expected_len.try_into().expect("expected_len too big");

        // Call sysctl() again to put the result in the vec
        check(sysctl_get(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            vec.as_mut_ptr() as *mut libc::c_void,
            &mut data_len,
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
        check(sysctl_get(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            ptr::null_mut(),
            &mut data_len,
        ))?;

        // Now allocate the Vec and set data_len to the real number of bytes allocated
        let mut vec: Vec<u8> = Vec::with_capacity(data_len);
        data_len = vec.capacity();

        // Call sysctl() again to put the result in the vec
        check(sysctl_get(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            vec.as_mut_ptr() as *mut libc::c_void,
            &mut data_len,
        ))?;

        // If that was ok, we can set the actual length of the vec to whatever
        // data_len was changed to, since that should now all be properly initialized data.
        vec.set_len(data_len);

        // On OpenBSD we have to do an extra step, because it fills the buffer with pointers to the
        // strings first, even though the strings are within the buffer as well.
        #[cfg(target_os = "openbsd")]
        let vec = {
            use std::ffi::CStr;

            // Set up some bounds checking. We assume there will be some pointers at the base until
            // we reach NULL, but we want to make sure we only ever read data within the range of
            // min_ptr..max_ptr.
            let ptrs = vec.as_ptr() as *const *const u8;
            let min_ptr = vec.as_ptr() as *const u8;
            let max_ptr = vec.as_ptr().add(vec.len()) as *const u8;
            let max_index: isize = (vec.len() / mem::size_of::<*const u8>())
                .try_into()
                .expect("too big for isize");

            let mut new_vec = Vec::with_capacity(vec.len());
            for index in 0..max_index {
                let ptr = ptrs.offset(index);
                if *ptr == ptr::null() {
                    break;
                } else {
                    // Make sure it's within the bounds of the buffer
                    assert!(
                        *ptr >= min_ptr && *ptr < max_ptr,
                        "pointer out of bounds of the buffer returned by sysctl()"
                    );
                    // Also bounds-check the C strings, to make sure we don't overrun the buffer
                    new_vec.extend(
                        CStr::from_bytes_until_nul(std::slice::from_raw_parts(
                            *ptr,
                            max_ptr.offset_from(*ptr) as usize,
                        ))
                        .expect("invalid C string")
                        .to_bytes_with_nul(),
                    );
                }
            }
            new_vec
        };

        Ok(vec)
    }
}

/// The working directory, from the `kern.proc_args.<pid>.cwd` (NetBSD) or `kern.proc_cwd.<pid>`
/// (OpenBSD) sysctl.
fn get_cwd(pid: i32) -> Option<String> {
    #[cfg(target_os = "netbsd")]
    let ctl_name = [CTL_KERN, KERN_PROC_ARGS, pid, KERN_PROC_CWD];
    #[cfg(target_os = "openbsd")]
    let ctl_name = [CTL_KERN, libc::KERN_PROC_CWD, pid];
    let mut buf = vec![0u8; libc::PATH_MAX as usize];
    let mut len = buf.len();
    // SAFETY: `buf` is writable for `len` bytes.
    check(unsafe {
        sysctl_get(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            buf.as_mut_ptr().cast(),
            &mut len,
        )
    })
    .ok()?;
    c_string(buf.get(..len)?)
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
        sysctl_get(
            ctl_name.as_ptr(),
            ctl_name.len() as u32,
            value.as_mut_ptr() as *mut libc::c_void,
            &mut value_len,
        )
    })?;
    assert_eq!(
        value_len,
        mem::size_of_val(&value),
        "Data requested from `sysctl` diverged in size from the expected return type. For variable length data you need to manually truncate the data to the valid returned size!"
    );
    Ok(unsafe { value.assume_init() })
}

fn get_pagesize() -> io::Result<libc::c_int> {
    // not in libc for some reason
    const HW_PAGESIZE: i32 = 7;
    unsafe { get_ctl(&[CTL_HW, HW_PAGESIZE]) }
}

/// Names a process state (`p_stat`), from `<sys/lwp.h>` on NetBSD and `<sys/proc.h>` on
/// OpenBSD. The names given here are the NetBSD ones, starting with LS*; the OpenBSD ones are the
/// same, just starting with S* instead.
fn process_status(stat: i8) -> &'static str {
    match stat {
        1 /* LSIDL */ => "",
        2 /* LSRUN */ => "Waiting",
        3 /* LSSLEEP */ => "Sleeping",
        4 /* LSSTOP */ => "Stopped",
        5 /* LSZOMB */ => "Zombie",
        // OpenBSD's zombies are SDEAD (its SZOMB is unused), which its `ps` shows as `Z`.
        #[cfg(target_os = "openbsd")] // removed in NetBSD
        6 /* LSDEAD */ => "Zombie",
        7 /* LSONPROC */ => "Running",
        #[cfg(target_os = "netbsd")] // doesn't exist in OpenBSD
        8 /* LSSUSPENDED */ => "Suspended",
        _ => "Unknown",
    }
}
