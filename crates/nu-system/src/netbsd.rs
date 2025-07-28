//! This is used for both NetBSD and OpenBSD, because they are fairly similar.

use itertools::{EitherOrBoth, Itertools};
use libc::{CTL_HW, CTL_KERN, KERN_PROC_ALL, KERN_PROC_ARGS, KERN_PROC_ARGV, sysctl};
use std::{
    io,
    mem::{self, MaybeUninit},
    ptr,
    time::{Duration, Instant},
};

#[cfg(target_os = "netbsd")]
type KInfoProc = libc::kinfo_proc2;
#[cfg(target_os = "openbsd")]
type KInfoProc = libc::kinfo_proc;

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub argv: Vec<u8>,
    pub stat: i8,
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

    // Join the processes between the two snapshots
    Ok(procs_a
        .into_iter()
        .merge_join_by(procs_b.into_iter(), |a, b| a.p_pid.cmp(&b.p_pid))
        .map(|proc| {
            // Take both snapshotted processes if we can, but if not then just keep the one that
            // exists and set prev_proc to None
            let (prev_proc, proc) = match proc {
                EitherOrBoth::Both(a, b) => (Some(a), b),
                EitherOrBoth::Left(a) => (None, a),
                EitherOrBoth::Right(b) => (None, b),
            };

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
                stat: proc.p_stat,
                percent_cpu,
                mem_resident: proc.p_vm_rssize.max(0) as u64 * pagesize,
                #[cfg(target_os = "netbsd")]
                mem_virtual: proc.p_vm_msize.max(0) as u64 * pagesize,
                #[cfg(target_os = "openbsd")]
                mem_virtual: proc.p_vm_map_size.max(0) as u64 * pagesize,
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
        // see sys/proc.h (OpenBSD), sys/lwp.h (NetBSD)
        // the names given here are the NetBSD ones, starting with LS*, but the OpenBSD ones are
        // the same, just starting with S* instead
        match self.stat {
            1 /* LSIDL */ => "",
            2 /* LSRUN */ => "Waiting",
            3 /* LSSLEEP */ => "Sleeping",
            4 /* LSSTOP */ => "Stopped",
            5 /* LSZOMB */ => "Zombie",
            #[cfg(target_os = "openbsd")] // removed in NetBSD
            6 /* LSDEAD */ => "Dead",
            7 /* LSONPROC */ => "Running",
            #[cfg(target_os = "netbsd")] // doesn't exist in OpenBSD
            8 /* LSSUSPENDED */ => "Suspended",
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
