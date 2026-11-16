// Attribution: a lot of this came from procs https://github.com/dalance/procs
// and sysinfo https://github.com/GuillaumeGomez/sysinfo

use crate::process::{ProcessInfo, command_line};
use libc::c_void;

use ntapi::ntexapi::{
    NtQuerySystemInformation, SYSTEM_PROCESS_INFORMATION, SYSTEM_THREAD_INFORMATION,
    SystemProcessInformation,
};
use ntapi::ntkeapi;
use ntapi::ntrtl::RTL_USER_PROCESS_PARAMETERS;
use ntapi::ntwow64::{PEB32, RTL_USER_PROCESS_PARAMETERS32};

use nu_utils::time::Instant;
use std::collections::HashMap;
use std::ffi::OsString;
use std::mem::{MaybeUninit, size_of, zeroed};
use std::os::windows::ffi::OsStringExt;
use std::ptr;
use std::ptr::null_mut;
use std::sync::LazyLock;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use windows::core::{Owned, PCWSTR, PWSTR};

use windows::Wdk::System::SystemServices::RtlGetVersion;
use windows::Wdk::System::Threading::{
    NtQueryInformationProcess, PROCESSINFOCLASS, ProcessBasicInformation,
    ProcessCommandLineInformation, ProcessWow64Information,
};

use windows::Win32::Foundation::{
    FALSE, HANDLE, HLOCAL, STATUS_BUFFER_OVERFLOW, STATUS_BUFFER_TOO_SMALL,
    STATUS_INFO_LENGTH_MISMATCH, UNICODE_STRING,
};

use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{
    AdjustTokenPrivileges, GetTokenInformation, LookupAccountSidW, LookupPrivilegeValueW, PSID,
    SE_DEBUG_NAME, SE_PRIVILEGE_ENABLED, SID_NAME_USE, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
    TOKEN_QUERY, TOKEN_USER, TokenUser,
};

use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;

use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, VirtualQueryEx};

use windows::Win32::System::SystemInformation::OSVERSIONINFOEXW;

use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PEB, PROCESS_ACCESS_RIGHTS,
    PROCESS_BASIC_INFORMATION, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ, QueryFullProcessImageNameW,
};

use windows::Win32::UI::Shell::CommandLineToArgvW;

/// Lists every process, measuring CPU usage over `interval`. When `long` is false, the details
/// only `ps --long` shows aren't read, so they are `None`.
///
/// Everything but the user, executable path, command line, environment, working directory and
/// console process group comes from a system-wide snapshot that covers every process, including
/// protected ones. Those six need a handle to the process, so they are missing for processes we
/// aren't allowed to open.
pub fn collect_proc(interval: Duration, long: bool) -> Vec<ProcessInfo> {
    let _ = set_privilege();

    let prev = process_snapshot();
    let prev_time = Instant::now();
    thread::sleep(interval);
    let curr = process_snapshot();
    let interval = Instant::now().saturating_duration_since(prev_time);

    let prev: HashMap<i32, (Option<SystemTime>, Option<Duration>)> = prev
        .into_iter()
        .map(|p| (p.pid, (p.start_time, p.cpu_time)))
        .collect();

    let mut user_names = HashMap::new();
    curr.into_iter()
        // The System Idle Process is the CPUs' idle time rather than a process.
        .filter(|p| p.pid != 0)
        .map(|mut p| {
            p.cpu_usage = prev
                .get(&p.pid)
                // A different creation time means the pid was reused.
                .filter(|(start_time, _)| *start_time == p.start_time)
                .and_then(|&(_, prev_cpu)| Some(p.cpu_time?.saturating_sub(prev_cpu?)))
                .map(|used| used.as_secs_f64() * 100.0 / interval.as_secs_f64());
            read_details(&mut p, long, &mut user_names);
            p
        })
        .collect()
}

/// Lists every process with `NtQuerySystemInformation(SystemProcessInformation)`. Unlike
/// opening each process, this works for protected and other users' processes too.
fn process_snapshot() -> Vec<ProcessInfo> {
    // `u64` elements keep the entries 8-byte aligned.
    let mut buffer: Vec<u64> = Vec::new();
    let mut len = 0u32;
    // Processes can start between the size query and the read, so retry a few times.
    for _ in 0..8 {
        let capacity = buffer.len() * size_of::<u64>();
        // SAFETY: the buffer is `capacity` bytes long.
        let status = unsafe {
            NtQuerySystemInformation(
                SystemProcessInformation,
                buffer.as_mut_ptr().cast(),
                capacity as u32,
                &mut len,
            )
        };
        if status == STATUS_INFO_LENGTH_MISMATCH.0 {
            let wanted = len as usize + 64 * 1024;
            buffer.resize(wanted.div_ceil(size_of::<u64>()), 0);
            continue;
        }
        if status < 0 {
            return Vec::new();
        }
        // The entries end with `NextEntryOffset == 0`, so the buffer size is enough of a bound.
        return parse_snapshot(&buffer, capacity);
    }
    Vec::new()
}

/// Walks the `SYSTEM_PROCESS_INFORMATION` entries the kernel wrote to the first `len` bytes of
/// `buffer`. Thread records or an image name that would reach past `len` are left out. The details
/// that need a handle to the process, and the CPU usage, which needs two snapshots, are `None`.
fn parse_snapshot(buffer: &[u64], len: usize) -> Vec<ProcessInfo> {
    // Every pointer into the buffer is derived from `base`, which may read all of it. `Threads`
    // is declared with one element, so a pointer taken from an entry's `Threads` field may not
    // read the other `NumberOfThreads - 1` records the kernel wrote after it.
    let base = buffer.as_ptr().cast::<u8>();
    let mut entries = Vec::new();
    let mut offset = 0;
    while offset + size_of::<SYSTEM_PROCESS_INFORMATION>() <= len {
        // SAFETY: the entry is inside the buffer (checked above) and 8-byte aligned, because the
        // buffer is and the kernel rounds every entry up to a multiple of 8 bytes.
        let info = unsafe { &*base.add(offset).cast::<SYSTEM_PROCESS_INFORMATION>() };
        let threads_start = offset + std::mem::offset_of!(SYSTEM_PROCESS_INFORMATION, Threads);
        let thread_count = info.NumberOfThreads as usize;
        let threads = if thread_count
            .checked_mul(size_of::<SYSTEM_THREAD_INFORMATION>())
            .is_some_and(|size| size <= len - threads_start)
        {
            // SAFETY: the records are inside the buffer (checked above) and aligned like the
            // entry.
            unsafe {
                std::slice::from_raw_parts(
                    base.add(threads_start).cast::<SYSTEM_THREAD_INFORMATION>(),
                    thread_count,
                )
            }
        } else {
            &[]
        };
        // SAFETY: these `LARGE_INTEGER` unions are plain 64-bit integers.
        let (user_time, kernel_time, create_time, private_working_set, read_bytes, write_bytes) = unsafe {
            (
                *info.UserTime.QuadPart(),
                *info.KernelTime.QuadPart(),
                *info.CreateTime.QuadPart(),
                *info.WorkingSetPrivateSize.QuadPart(),
                *info.ReadTransferCount.QuadPart(),
                *info.WriteTransferCount.QuadPart(),
            )
        };
        // The kernel copies the name into the buffer after the thread records and points
        // `ImageName.Buffer` at the copy. It's null for the System Idle Process.
        let name_bytes = usize::from(info.ImageName.Length);
        let name = (info.ImageName.Buffer as usize)
            .checked_sub(base as usize)
            .filter(|&start| {
                start % 2 == 0
                    && len
                        .checked_sub(start)
                        .is_some_and(|room| name_bytes <= room)
            })
            .map(|start| {
                // SAFETY: the name's `Length` bytes are inside the buffer (checked above) and
                // aligned for UTF-16.
                String::from_utf16_lossy(unsafe {
                    std::slice::from_raw_parts(base.add(start).cast::<u16>(), name_bytes / 2)
                })
            })
            .unwrap_or_default();
        let base_priority = i64::from(info.BasePriority);
        entries.push(ProcessInfo {
            pid: info.UniqueProcessId as usize as i32,
            ppid: info.InheritedFromUniqueProcessId as usize as i32,
            name,
            command: None,
            exe: None,
            user: None,
            user_id: None,
            status: process_status(threads),
            cpu_usage: None,
            // `UserTime` and `KernelTime` count 100ns units.
            cpu_time: Some(Duration::from_nanos(
                ((user_time + kernel_time) as u64).saturating_mul(100),
            )),
            mem_size: Some(info.WorkingSetSize as u64),
            virtual_size: Some(info.VirtualSize as u64),
            // The private working set: resident memory that isn't shared with other processes.
            // Task Manager shows this as "Memory".
            private_size: Some(private_working_set as u64),
            // These count all of the process's I/O: files, devices, pipes and network.
            disk_read: Some(read_bytes as u64),
            disk_written: Some(write_bytes as u64),
            start_time: filetime_to_system_time(create_time),
            process_group_id: None,
            // The Terminal Services session
            session_id: Some(info.SessionId.into()),
            // The base priority, e.g. 8 for normal priority
            priority: Some(base_priority),
            // The priority class on the unix nice scale, mapped the way libuv (and Node.js's
            // `os.getPriority()`) does
            nice: Some(nice_from_base_priority(base_priority)),
            thread_count: Some(info.NumberOfThreads.into()),
            cwd: None,
            environ: None,
        });
        if info.NextEntryOffset == 0 {
            break;
        }
        offset += info.NextEntryOffset as usize;
    }
    entries
}

/// Derives a process state from its threads, with the same names as on unix: "Running" if any
/// thread is running or ready to run, "Suspended" if every thread is suspended, else "Sleeping".
fn process_status(threads: &[SYSTEM_THREAD_INFORMATION]) -> Option<&'static str> {
    if threads.is_empty() {
        return None;
    }
    let running = threads.iter().any(|t| {
        matches!(
            t.ThreadState,
            ntkeapi::Running | ntkeapi::Ready | ntkeapi::Standby | ntkeapi::DeferredReady
        )
    });
    let suspended = threads
        .iter()
        .all(|t| t.ThreadState == ntkeapi::Waiting && t.WaitReason == ntkeapi::Suspended);
    Some(if running {
        "Running"
    } else if suspended {
        "Suspended"
    } else {
        "Sleeping"
    })
}

/// Converts a `FILETIME` (100ns units since 1601-01-01 UTC) to a `SystemTime`.
fn filetime_to_system_time(filetime: i64) -> Option<SystemTime> {
    // 100ns intervals between 1601-01-01 and the unix epoch, 1970-01-01.
    const UNIX_EPOCH_AS_FILETIME: i64 = 116_444_736_000_000_000;
    let since_epoch = u64::try_from(filetime.checked_sub(UNIX_EPOCH_AS_FILETIME)?).ok()?;
    Some(UNIX_EPOCH + Duration::from_nanos(since_epoch.checked_mul(100)?))
}

/// Opens a handle to the process with `access`, which closes when it drops.
fn open_process(pid: i32, access: PROCESS_ACCESS_RIGHTS) -> Option<Owned<HANDLE>> {
    // SAFETY: `OpenProcess` takes only plain values, and fails for a pid we can't open.
    let handle = unsafe { OpenProcess(access, false, pid as u32) }.ok()?;
    // SAFETY: the handle is new, so nothing else closes it.
    Some(unsafe { Owned::new(handle) })
}

/// Fills in what needs a handle to the process: the user and, when `long` is true, the
/// executable path, command line, environment, working directory and console process group.
fn read_details(
    info: &mut ProcessInfo,
    long: bool,
    user_names: &mut HashMap<String, Option<String>>,
) {
    // The environment, working directory and process group are in the process's memory, which
    // needs VM_READ. The limited right is granted for more processes (elevated ones, for
    // example) and is enough for the user, the executable and, on Windows 8.1 and newer, the
    // command line.
    let full_access = long
        .then(|| open_process(info.pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ))
        .flatten();
    let has_vm_read = full_access.is_some();
    let Some(handle) =
        full_access.or_else(|| open_process(info.pid, PROCESS_QUERY_LIMITED_INFORMATION))
    else {
        return;
    };
    if let Some((sid, name)) = get_user(*handle, user_names) {
        info.user_id = Some(sid);
        info.user = name;
    }
    if !long {
        return;
    }
    info.exe = get_exe(*handle);
    // SAFETY: `handle` has VM_READ access whenever `has_vm_read` is set, and it stays open until
    // it drops at the end of this function.
    if !(has_vm_read && unsafe { read_process_params(*handle, info) }.is_ok()) {
        info.command = command_line(&query_cmd_line(*handle));
    }
}

/// Full path of the process's executable. Needs only `PROCESS_QUERY_LIMITED_INFORMATION`.
fn get_exe(handle: HANDLE) -> Option<String> {
    // Long-path aware executables can live under paths longer than `MAX_PATH`.
    let mut buffer = vec![0u16; 32 * 1024];
    let mut len = buffer.len() as u32;
    // SAFETY: `buffer` holds `len` UTF-16 units. On success `len` is the number written, not
    // counting the terminating NUL.
    unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR::from_raw(buffer.as_mut_ptr()),
            &mut len,
        )
    }
    .ok()?;
    Some(String::from_utf16_lossy(buffer.get(..len as usize)?))
}

/// Enables `SeDebugPrivilege` for nushell, when its user has it (administrators do), so that it
/// can open other users' processes.
fn set_privilege() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &mut token).is_err() {
            return false;
        }
        // The token closes when it drops.
        let token = Owned::new(token);

        let mut tps: TOKEN_PRIVILEGES = zeroed();
        tps.PrivilegeCount = 1;
        LookupPrivilegeValueW(PCWSTR::null(), SE_DEBUG_NAME, &mut tps.Privileges[0].Luid).is_ok()
            && {
                tps.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
                AdjustTokenPrivileges(*token, FALSE.into(), Some(&tps), 0, None, None).is_ok()
            }
    }
}

trait RtlUserProcessParameters {
    fn get_cmdline(&self, handle: HANDLE) -> Result<Vec<u16>, &'static str>;
    fn get_cwd(&self, handle: HANDLE) -> Result<Vec<u16>, &'static str>;
    fn get_environ(&self, handle: HANDLE) -> Result<Vec<u16>, &'static str>;
    fn process_group_id(&self) -> u32;
}

macro_rules! impl_RtlUserProcessParameters {
    ($t:ty) => {
        impl RtlUserProcessParameters for $t {
            fn get_cmdline(&self, handle: HANDLE) -> Result<Vec<u16>, &'static str> {
                let ptr = self.CommandLine.Buffer;
                let size = self.CommandLine.Length;
                unsafe { get_process_data(handle, ptr as _, size as _) }
            }
            fn get_cwd(&self, handle: HANDLE) -> Result<Vec<u16>, &'static str> {
                let ptr = self.CurrentDirectory.DosPath.Buffer;
                let size = self.CurrentDirectory.DosPath.Length;
                unsafe { get_process_data(handle, ptr as _, size as _) }
            }
            fn get_environ(&self, handle: HANDLE) -> Result<Vec<u16>, &'static str> {
                let ptr = self.Environment;
                unsafe {
                    let size = get_region_size(handle, ptr as _)?;
                    get_process_data(handle, ptr as _, size as _)
                }
            }
            fn process_group_id(&self) -> u32 {
                self.ProcessGroupId
            }
        }
    };
}

impl_RtlUserProcessParameters!(RTL_USER_PROCESS_PARAMETERS32);
impl_RtlUserProcessParameters!(RTL_USER_PROCESS_PARAMETERS);

unsafe fn null_terminated_wchar_to_string(slice: &[u16]) -> String {
    match slice.iter().position(|&x| x == 0) {
        Some(pos) => OsString::from_wide(&slice[..pos])
            .to_string_lossy()
            .into_owned(),
        None => OsString::from_wide(slice).to_string_lossy().into_owned(),
    }
}

unsafe fn get_process_data(
    handle: HANDLE,
    ptr: *const c_void,
    size: usize,
) -> Result<Vec<u16>, &'static str> {
    let mut buffer: Vec<u16> = Vec::with_capacity(size / 2 + 1);
    let mut bytes_read = 0;

    unsafe {
        if ReadProcessMemory(
            handle,
            ptr,
            buffer.as_mut_ptr().cast(),
            size,
            Some(&mut bytes_read),
        )
        .is_err()
        {
            return Err("Unable to read process data");
        }

        // Documentation states that the function fails if not all data is accessible.
        if bytes_read != size {
            return Err("ReadProcessMemory returned unexpected number of bytes read");
        }

        buffer.set_len(size / 2);
        buffer.push(0);
    }

    Ok(buffer)
}

unsafe fn get_region_size(handle: HANDLE, ptr: *const c_void) -> Result<usize, &'static str> {
    unsafe {
        let mut meminfo = MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();
        if VirtualQueryEx(
            handle,
            Some(ptr),
            meminfo.as_mut_ptr().cast(),
            size_of::<MEMORY_BASIC_INFORMATION>(),
        ) == 0
        {
            return Err("Unable to read process memory information");
        }
        let meminfo = meminfo.assume_init();
        Ok((meminfo.RegionSize as isize - ptr.offset_from(meminfo.BaseAddress)) as usize)
    }
}

unsafe fn ph_query_process_variable_size(
    process_handle: HANDLE,
    process_information_class: PROCESSINFOCLASS,
) -> Option<Vec<u16>> {
    unsafe {
        let mut return_length = MaybeUninit::<u32>::uninit();

        if let Err(err) = NtQueryInformationProcess(
            process_handle,
            process_information_class,
            std::ptr::null_mut(),
            0,
            return_length.as_mut_ptr() as *mut _,
        )
        .ok()
            && ![
                STATUS_BUFFER_OVERFLOW.into(),
                STATUS_BUFFER_TOO_SMALL.into(),
                STATUS_INFO_LENGTH_MISMATCH.into(),
            ]
            .contains(&err.code())
        {
            return None;
        }

        let mut return_length = return_length.assume_init();
        let buf_len = (return_length as usize) / 2;
        let mut buffer: Vec<u16> = Vec::with_capacity(buf_len + 1);
        if NtQueryInformationProcess(
            process_handle,
            process_information_class,
            buffer.as_mut_ptr() as *mut _,
            return_length,
            &mut return_length as *mut _,
        )
        .is_err()
        {
            return None;
        }
        buffer.set_len(buf_len);
        buffer.push(0);
        Some(buffer)
    }
}

unsafe fn get_cmdline_from_buffer(buffer: PCWSTR) -> Vec<String> {
    unsafe {
        // `CommandLineToArgvW` returns the path of the current executable (nushell) for an empty
        // command line.
        if buffer.is_null() || *buffer.0 == 0 {
            return Vec::new();
        }
        // Get argc and argv from the command line
        let mut argc = MaybeUninit::<i32>::uninit();
        let argv_p = CommandLineToArgvW(buffer, argc.as_mut_ptr());
        if argv_p.is_null() {
            return Vec::new();
        }
        // The array is ours to free with `LocalFree`, which dropping it does.
        let _argv_memory = Owned::new(HLOCAL(argv_p.cast()));
        let argc = argc.assume_init();
        let argv = std::slice::from_raw_parts(argv_p, argc as usize);
        argv.iter()
            .map(|arg| String::from_utf16_lossy(arg.as_wide()))
            .collect()
    }
}

/// Fills in the command line, environment, working directory and process group from the
/// process's memory. Nothing is filled in when that memory can't be read.
///
/// The process group is `RTL_USER_PROCESS_PARAMETERS.ProcessGroupId`, which Windows 8 added.
/// Rust's Windows targets need Windows 10, so it is always there. (The struct's `Length` counts
/// the strings stored after it, so it can't tell struct versions apart.)
///
/// # Safety
///
/// `handle` must be an open process handle with `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`
/// access.
unsafe fn read_process_params(handle: HANDLE, info: &mut ProcessInfo) -> Result<(), &'static str> {
    unsafe {
        if !cfg!(target_pointer_width = "64") {
            return Err("Non 64 bit targets are not supported");
        }

        // First check if target process is running in wow64 compatibility emulator
        let mut pwow32info = MaybeUninit::<*const c_void>::uninit();
        if NtQueryInformationProcess(
            handle,
            ProcessWow64Information,
            pwow32info.as_mut_ptr().cast(),
            size_of::<*const c_void>() as u32,
            null_mut(),
        )
        .is_err()
        {
            return Err("Unable to check WOW64 information about the process");
        }
        let pwow32info = pwow32info.assume_init();

        if pwow32info.is_null() {
            // target is a 64 bit process

            let mut pbasicinfo = MaybeUninit::<PROCESS_BASIC_INFORMATION>::uninit();
            if NtQueryInformationProcess(
                handle,
                ProcessBasicInformation,
                pbasicinfo.as_mut_ptr().cast(),
                size_of::<PROCESS_BASIC_INFORMATION>() as u32,
                null_mut(),
            )
            .is_err()
            {
                return Err("Unable to get basic process information");
            }
            let pinfo = pbasicinfo.assume_init();

            let mut peb = MaybeUninit::<PEB>::uninit();
            if ReadProcessMemory(
                handle,
                pinfo.PebBaseAddress.cast(),
                peb.as_mut_ptr().cast(),
                size_of::<PEB>(),
                None,
            )
            .is_err()
            {
                return Err("Unable to read process PEB");
            }

            let peb = peb.assume_init();

            let mut proc_params = MaybeUninit::<RTL_USER_PROCESS_PARAMETERS>::uninit();
            if ReadProcessMemory(
                handle,
                peb.ProcessParameters.cast(),
                proc_params.as_mut_ptr().cast(),
                size_of::<RTL_USER_PROCESS_PARAMETERS>(),
                None,
            )
            .is_err()
            {
                return Err("Unable to read process parameters");
            }

            fill_from_params(info, &proc_params.assume_init(), handle);
            return Ok(());
        }
        // target is a 32 bit process in wow64 mode

        let mut peb32 = MaybeUninit::<PEB32>::uninit();
        if ReadProcessMemory(
            handle,
            pwow32info,
            peb32.as_mut_ptr().cast(),
            size_of::<PEB32>(),
            None,
        )
        .is_err()
        {
            return Err("Unable to read PEB32");
        }
        let peb32 = peb32.assume_init();

        let mut proc_params = MaybeUninit::<RTL_USER_PROCESS_PARAMETERS32>::uninit();
        if ReadProcessMemory(
            handle,
            peb32.ProcessParameters as *mut _,
            proc_params.as_mut_ptr().cast(),
            size_of::<RTL_USER_PROCESS_PARAMETERS32>(),
            None,
        )
        .is_err()
        {
            return Err("Unable to read 32 bit process parameters");
        }
        fill_from_params(info, &proc_params.assume_init(), handle);
        Ok(())
    }
}

/// Fills in what `read_process_params` reads through a process's `RTL_USER_PROCESS_PARAMETERS`.
fn fill_from_params<T: RtlUserProcessParameters>(
    info: &mut ProcessInfo,
    params: &T,
    handle: HANDLE,
) {
    info.command = command_line(&get_cmd_line(params, handle));
    info.environ = get_proc_env(params, handle);
    info.cwd = get_cwd(params, handle);
    info.process_group_id = Some(params.process_group_id() as i32);
}

static WINDOWS_8_1_OR_NEWER: LazyLock<bool> = LazyLock::new(|| unsafe {
    let mut version_info: OSVERSIONINFOEXW = MaybeUninit::zeroed().assume_init();

    version_info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;
    if RtlGetVersion((&mut version_info as *mut OSVERSIONINFOEXW).cast()).is_err() {
        return true;
    }

    // Windows 8.1 is 6.3
    version_info.dwMajorVersion > 6
        || version_info.dwMajorVersion == 6 && version_info.dwMinorVersion >= 3
});

fn get_cmd_line<T: RtlUserProcessParameters>(params: &T, handle: HANDLE) -> Vec<String> {
    if *WINDOWS_8_1_OR_NEWER {
        get_cmd_line_new(handle)
    } else {
        get_cmd_line_old(params, handle)
    }
}

/// Reads the command line without reading the process's memory, which only needs
/// `PROCESS_QUERY_LIMITED_INFORMATION`. Empty before Windows 8.1, which doesn't support it.
fn query_cmd_line(handle: HANDLE) -> Vec<String> {
    if *WINDOWS_8_1_OR_NEWER {
        get_cmd_line_new(handle)
    } else {
        Vec::new()
    }
}

#[allow(clippy::cast_ptr_alignment)]
fn get_cmd_line_new(handle: HANDLE) -> Vec<String> {
    unsafe {
        if let Some(buffer) = ph_query_process_variable_size(handle, ProcessCommandLineInformation)
        {
            let buffer = (*(buffer.as_ptr() as *const UNICODE_STRING)).Buffer;

            get_cmdline_from_buffer(PCWSTR::from_raw(buffer.as_ptr()))
        } else {
            Vec::new()
        }
    }
}

fn get_cmd_line_old<T: RtlUserProcessParameters>(params: &T, handle: HANDLE) -> Vec<String> {
    match params.get_cmdline(handle) {
        Ok(buffer) => unsafe { get_cmdline_from_buffer(PCWSTR::from_raw(buffer.as_ptr())) },
        Err(_e) => Vec::new(),
    }
}

fn get_proc_env<T: RtlUserProcessParameters>(params: &T, handle: HANDLE) -> Option<Vec<String>> {
    match params.get_environ(handle) {
        Ok(buffer) => {
            let equals = "="
                .encode_utf16()
                .next()
                .expect("unable to get next utf16 value");
            let raw_env = buffer;
            let mut result = Vec::new();
            let mut begin = 0;
            while let Some(offset) = raw_env[begin..].iter().position(|&c| c == 0) {
                let end = begin + offset;
                if raw_env[begin..end].contains(&equals) {
                    result.push(
                        OsString::from_wide(&raw_env[begin..end])
                            .to_string_lossy()
                            .into_owned(),
                    );
                    begin = end + 1;
                } else {
                    break;
                }
            }
            Some(result)
        }
        Err(_e) => None,
    }
}

fn get_cwd<T: RtlUserProcessParameters>(params: &T, handle: HANDLE) -> Option<String> {
    match params.get_cwd(handle) {
        Ok(buffer) => Some(unsafe { null_terminated_wchar_to_string(buffer.as_slice()) }),
        Err(_e) => None,
    }
}

/// Looks up the user a process runs as: its SID string (e.g. "S-1-5-18") and account name.
/// Account names are cached by SID in `user_names`, because looking one up can be slow (it may
/// ask a domain controller).
fn get_user(
    handle: HANDLE,
    user_names: &mut HashMap<String, Option<String>>,
) -> Option<(String, Option<String>)> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(handle, TOKEN_QUERY, &mut token).ok()?;
        // The token closes when it drops.
        let token = Owned::new(token);

        let mut cb_needed = 0;
        let _ = GetTokenInformation(
            *token,
            TokenUser,
            Some(ptr::null::<c_void>() as *mut c_void),
            0,
            &mut cb_needed,
        );

        let mut buf: Vec<u8> = Vec::with_capacity(cb_needed as usize);

        GetTokenInformation(
            *token,
            TokenUser,
            Some(buf.as_mut_ptr() as *mut c_void),
            cb_needed,
            &mut cb_needed,
        )
        .ok()?;
        buf.set_len(cb_needed as usize);

        #[allow(clippy::cast_ptr_alignment)]
        let token_user = buf.as_ptr() as *const TOKEN_USER;
        let psid = (*token_user).User.Sid;

        let sid = sid_string(psid)?;
        let name = user_names
            .entry(sid.clone())
            .or_insert_with(|| get_name(psid))
            .clone();

        Some((sid, name))
    }
}

/// Formats a SID as a string such as "S-1-5-18".
///
/// # Safety
///
/// `psid` must point to a valid SID.
unsafe fn sid_string(psid: PSID) -> Option<String> {
    let mut string = PWSTR::null();
    // SAFETY: the caller guarantees that `psid` is valid. On success `string` points to a
    // NUL-terminated string that the system allocated with `LocalAlloc`.
    unsafe { ConvertSidToStringSidW(psid, &mut string) }.ok()?;
    // SAFETY: the string is ours to free with `LocalFree`, which dropping this does.
    let _string_memory = unsafe { Owned::new(HLOCAL(string.0.cast())) };
    // SAFETY: `string` stays allocated until `_string_memory` drops.
    unsafe { string.to_string() }.ok()
}

/// Looks up the account name of a SID.
fn get_name(psid: PSID) -> Option<String> {
    unsafe {
        let mut cc_name = 0;
        let mut cc_domainname = 0;
        let mut pe_use = SID_NAME_USE::default();
        let _ = LookupAccountSidW(
            None,
            psid,
            None,
            &mut cc_name,
            None,
            &mut cc_domainname,
            &mut pe_use,
        );

        if cc_name == 0 || cc_domainname == 0 {
            return None;
        }

        let mut name = vec![0u16; cc_name as usize];
        let mut domainname = vec![0u16; cc_domainname as usize];
        LookupAccountSidW(
            None,
            psid,
            PWSTR::from_raw(name.as_mut_ptr()).into(),
            &mut cc_name,
            PWSTR::from_raw(domainname.as_mut_ptr()).into(),
            &mut cc_domainname,
            &mut pe_use,
        )
        .ok()?;

        // On success `cc_name` is the length of the name, not counting the terminating NUL.
        Some(String::from_utf16_lossy(name.get(..cc_name as usize)?))
    }
}

/// Maps a process's base priority to the nice value libuv gives its priority class: realtime
/// -20, high -14, above normal -7, normal 0, below normal 10 and idle 19. Each class has a fixed
/// base priority (24, 13, 10, 8, 6 and 4), so ranges between them keep a process whose base
/// priority was set directly in the nearest class.
fn nice_from_base_priority(base_priority: i64) -> i64 {
    match base_priority {
        ..=4 => 19,
        5..=6 => 10,
        7..=9 => 0,
        10..=11 => -7,
        12..=15 => -14,
        _ => -20,
    }
}

#[cfg(test)]
mod tests {
    use super::nice_from_base_priority;

    #[test]
    fn priority_classes_map_to_libuv_nice_values() {
        // Base priorities of the idle, below normal, normal, above normal, high and realtime
        // priority classes.
        let nice: Vec<i64> = [4, 6, 8, 10, 13, 24]
            .into_iter()
            .map(nice_from_base_priority)
            .collect();
        assert_eq!(nice, [19, 10, 0, -7, -14, -20]);
    }
}
