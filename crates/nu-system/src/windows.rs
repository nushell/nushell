// Attribution: a lot of this came from procs https://github.com/dalance/procs
// and sysinfo https://github.com/GuillaumeGomez/sysinfo

use chrono::offset::TimeZone;
use chrono::{Local, NaiveDate};
use libc::c_void;
use ntapi::ntpebteb::PEB;
use ntapi::ntpsapi::{
    NtQueryInformationProcess, ProcessBasicInformation, ProcessCommandLineInformation,
    ProcessWow64Information, PROCESSINFOCLASS, PROCESS_BASIC_INFORMATION,
};
use ntapi::ntrtl::{RtlGetVersion, PRTL_USER_PROCESS_PARAMETERS, RTL_USER_PROCESS_PARAMETERS};
use ntapi::ntwow64::{PEB32, PRTL_USER_PROCESS_PARAMETERS32, RTL_USER_PROCESS_PARAMETERS32};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::mem::{size_of, zeroed, MaybeUninit};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr;
use std::ptr::null_mut;
use std::thread;
use std::time::{Duration, Instant};
use winapi::shared::basetsd::SIZE_T;
use winapi::shared::minwindef::{DWORD, FALSE, FILETIME, LPVOID, MAX_PATH, TRUE, ULONG};
use winapi::shared::ntdef::{NT_SUCCESS, UNICODE_STRING};
use winapi::shared::ntstatus::{
    STATUS_BUFFER_OVERFLOW, STATUS_BUFFER_TOO_SMALL, STATUS_INFO_LENGTH_MISMATCH,
};
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi::{ReadProcessMemory, VirtualQueryEx};
use winapi::um::processthreadsapi::{
    GetCurrentProcess, GetPriorityClass, GetProcessTimes, OpenProcess, OpenProcessToken,
};
use winapi::um::psapi::{
    GetModuleBaseNameW, GetProcessMemoryInfo, K32EnumProcesses, PROCESS_MEMORY_COUNTERS,
    PROCESS_MEMORY_COUNTERS_EX,
};
use winapi::um::securitybaseapi::{AdjustTokenPrivileges, GetTokenInformation};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use winapi::um::winbase::{GetProcessIoCounters, LookupAccountSidW, LookupPrivilegeValueW};
use winapi::um::winnt::{
    TokenGroups, TokenUser, HANDLE, IO_COUNTERS, MEMORY_BASIC_INFORMATION,
    PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, PSID, RTL_OSVERSIONINFOEXW, SE_DEBUG_NAME,
    SE_PRIVILEGE_ENABLED, SID, TOKEN_ADJUST_PRIVILEGES, TOKEN_GROUPS, TOKEN_PRIVILEGES,
    TOKEN_QUERY, TOKEN_USER,
};

pub struct ProcessInfo {
    pub pid: i32,
    pub command: String,
    pub ppid: i32,
    pub start_time: chrono::DateTime<chrono::Local>,
    pub cpu_info: CpuInfo,
    pub memory_info: MemoryInfo,
    pub disk_info: DiskInfo,
    pub user: SidName,
    pub groups: Vec<SidName>,
    pub priority: u32,
    pub thread: i32,
    pub interval: Duration,
    pub cmd: Vec<String>,
    pub environ: Vec<String>,
    pub cwd: PathBuf,
}

#[derive(Default)]
pub struct MemoryInfo {
    pub page_fault_count: u64,
    pub peak_working_set_size: u64,
    pub working_set_size: u64,
    pub quota_peak_paged_pool_usage: u64,
    pub quota_paged_pool_usage: u64,
    pub quota_peak_non_paged_pool_usage: u64,
    pub quota_non_paged_pool_usage: u64,
    pub page_file_usage: u64,
    pub peak_page_file_usage: u64,
    pub private_usage: u64,
}

#[derive(Default)]
pub struct DiskInfo {
    pub prev_read: u64,
    pub prev_write: u64,
    pub curr_read: u64,
    pub curr_write: u64,
}

#[derive(Default)]
pub struct CpuInfo {
    pub prev_sys: u64,
    pub prev_user: u64,
    pub curr_sys: u64,
    pub curr_user: u64,
}

#[cfg_attr(tarpaulin, skip)]
pub fn collect_proc(interval: Duration, _with_thread: bool) -> Vec<ProcessInfo> {
    let mut base_procs = Vec::new();
    let mut ret = Vec::new();

    let _ = set_privilege();

    for pid in get_pids() {
        let handle = get_handle(pid);

        if let Some(handle) = handle {
            let times = get_times(handle);
            let io = get_io(handle);

            let time = Instant::now();

            if let (Some((_, _, sys, user)), Some((read, write))) = (times, io) {
                base_procs.push((pid, sys, user, read, write, time));
            }
        }
    }

    thread::sleep(interval);

    let (mut ppids, mut threads) = get_ppid_threads();

    for (pid, prev_sys, prev_user, prev_read, prev_write, prev_time) in base_procs {
        let ppid = ppids.remove(&pid);
        let thread = threads.remove(&pid);
        let handle = get_handle(pid);

        if let Some(handle) = handle {
            let command = get_command(handle);
            let memory_info = get_memory_info(handle);
            let times = get_times(handle);
            let io = get_io(handle);

            let start_time = if let Some((start, _, _, _)) = times {
                let time = chrono::Duration::seconds(start as i64 / 10_000_000);
                let base = NaiveDate::from_ymd(1600, 1, 1).and_hms(0, 0, 0);
                let time = base + time;
                Local.from_utc_datetime(&time)
            } else {
                Local.from_utc_datetime(&NaiveDate::from_ymd(1600, 1, 1).and_hms(0, 0, 0))
            };

            let cpu_info = if let Some((_, _, curr_sys, curr_user)) = times {
                Some(CpuInfo {
                    prev_sys,
                    prev_user,
                    curr_sys,
                    curr_user,
                })
            } else {
                None
            };

            let disk_info = if let Some((curr_read, curr_write)) = io {
                Some(DiskInfo {
                    prev_read,
                    prev_write,
                    curr_read,
                    curr_write,
                })
            } else {
                None
            };

            let user = get_user(handle);
            let groups = get_groups(handle);

            let priority = get_priority(handle);

            let curr_time = Instant::now();
            let interval = curr_time - prev_time;

            let mut all_ok = true;
            all_ok &= command.is_some();
            all_ok &= cpu_info.is_some();
            all_ok &= memory_info.is_some();
            all_ok &= disk_info.is_some();
            all_ok &= user.is_some();
            all_ok &= groups.is_some();
            all_ok &= thread.is_some();

            if all_ok {
                // let process_params = unsafe { get_process_params(handle) };
                // match process_params {
                //     Ok((pp_cmd, pp_env, pp_cwd)) => {
                //         eprintln!(
                //             "cmd: {:?}, env: {:?}, cwd: {:?}",
                //             pp_cmd,
                //             "noop".to_string(),
                //             pp_cwd
                //         );
                //     }
                //     Err(_) => {}
                // }
                let (proc_cmd, proc_env, proc_cwd) = match unsafe { get_process_params(handle) } {
                    Ok(pp) => (pp.0, pp.1, pp.2),
                    Err(_) => (vec![], vec![], PathBuf::new()),
                };
                let command = command.unwrap_or_default();
                let ppid = ppid.unwrap_or(0);
                let cpu_info = cpu_info.unwrap_or_default();
                let memory_info = memory_info.unwrap_or_default();
                let disk_info = disk_info.unwrap_or_default();
                let user = user.unwrap_or_else(|| SidName {
                    sid: vec![],
                    name: None,
                    domainname: None,
                });
                let groups = groups.unwrap_or_default();
                let thread = thread.unwrap_or_default();

                let proc = ProcessInfo {
                    pid,
                    command,
                    ppid,
                    start_time,
                    cpu_info,
                    memory_info,
                    disk_info,
                    user,
                    groups,
                    priority,
                    thread,
                    interval,
                    cmd: proc_cmd,
                    environ: proc_env,
                    cwd: proc_cwd,
                };

                ret.push(proc);
            }

            unsafe {
                CloseHandle(handle);
            }
        }
    }

    ret
}

#[cfg_attr(tarpaulin, skip)]
fn set_privilege() -> bool {
    unsafe {
        let handle = GetCurrentProcess();
        let mut token: HANDLE = zeroed();
        let ret = OpenProcessToken(handle, TOKEN_ADJUST_PRIVILEGES, &mut token);
        if ret == 0 {
            return false;
        }

        let mut tps: TOKEN_PRIVILEGES = zeroed();
        let se_debug_name: Vec<u16> = format!("{}\0", SE_DEBUG_NAME).encode_utf16().collect();
        tps.PrivilegeCount = 1;
        let ret = LookupPrivilegeValueW(
            ptr::null(),
            se_debug_name.as_ptr(),
            &mut tps.Privileges[0].Luid,
        );
        if ret == 0 {
            return false;
        }

        tps.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        let ret = AdjustTokenPrivileges(
            token,
            FALSE,
            &mut tps,
            0,
            ptr::null::<TOKEN_PRIVILEGES>() as *mut TOKEN_PRIVILEGES,
            ptr::null::<u32>() as *mut u32,
        );
        if ret == 0 {
            return false;
        }

        true
    }
}

#[cfg_attr(tarpaulin, skip)]
fn get_pids() -> Vec<i32> {
    let dword_size = size_of::<DWORD>();
    let mut pids: Vec<DWORD> = Vec::with_capacity(10192);
    let mut cb_needed = 0;

    unsafe {
        pids.set_len(10192);
        let result = K32EnumProcesses(
            pids.as_mut_ptr(),
            (dword_size * pids.len()) as DWORD,
            &mut cb_needed,
        );
        if result == 0 {
            return Vec::new();
        }
        let pids_len = cb_needed / dword_size as DWORD;
        pids.set_len(pids_len as usize);
    }

    pids.iter().map(|x| *x as i32).collect()
}

#[cfg_attr(tarpaulin, skip)]
fn get_ppid_threads() -> (HashMap<i32, i32>, HashMap<i32, i32>) {
    let mut ppids = HashMap::new();
    let mut threads = HashMap::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        let mut entry: PROCESSENTRY32 = zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32>() as u32;
        let mut not_the_end = Process32First(snapshot, &mut entry);

        while not_the_end != 0 {
            ppids.insert(entry.th32ProcessID as i32, entry.th32ParentProcessID as i32);
            threads.insert(entry.th32ProcessID as i32, entry.cntThreads as i32);
            not_the_end = Process32Next(snapshot, &mut entry);
        }

        CloseHandle(snapshot);
    }

    (ppids, threads)
}

#[cfg_attr(tarpaulin, skip)]
fn get_handle(pid: i32) -> Option<HANDLE> {
    if pid == 0 {
        return None;
    }

    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            FALSE,
            pid as DWORD,
        )
    };

    if handle.is_null() {
        None
    } else {
        Some(handle)
    }
}

#[cfg_attr(tarpaulin, skip)]
fn get_times(handle: HANDLE) -> Option<(u64, u64, u64, u64)> {
    unsafe {
        let mut start: FILETIME = zeroed();
        let mut exit: FILETIME = zeroed();
        let mut sys: FILETIME = zeroed();
        let mut user: FILETIME = zeroed();

        let ret = GetProcessTimes(
            handle,
            &mut start as *mut FILETIME,
            &mut exit as *mut FILETIME,
            &mut sys as *mut FILETIME,
            &mut user as *mut FILETIME,
        );

        let start = u64::from(start.dwHighDateTime) << 32 | u64::from(start.dwLowDateTime);
        let exit = u64::from(exit.dwHighDateTime) << 32 | u64::from(exit.dwLowDateTime);
        let sys = u64::from(sys.dwHighDateTime) << 32 | u64::from(sys.dwLowDateTime);
        let user = u64::from(user.dwHighDateTime) << 32 | u64::from(user.dwLowDateTime);

        if ret != 0 {
            Some((start, exit, sys, user))
        } else {
            None
        }
    }
}

#[cfg_attr(tarpaulin, skip)]
fn get_memory_info(handle: HANDLE) -> Option<MemoryInfo> {
    unsafe {
        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = zeroed();
        let ret = GetProcessMemoryInfo(
            handle,
            &mut pmc as *mut PROCESS_MEMORY_COUNTERS_EX as *mut c_void
                as *mut PROCESS_MEMORY_COUNTERS,
            size_of::<PROCESS_MEMORY_COUNTERS_EX>() as DWORD,
        );

        if ret != 0 {
            let info = MemoryInfo {
                page_fault_count: u64::from(pmc.PageFaultCount),
                peak_working_set_size: pmc.PeakWorkingSetSize as u64,
                working_set_size: pmc.WorkingSetSize as u64,
                quota_peak_paged_pool_usage: pmc.QuotaPeakPagedPoolUsage as u64,
                quota_paged_pool_usage: pmc.QuotaPagedPoolUsage as u64,
                quota_peak_non_paged_pool_usage: pmc.QuotaPeakNonPagedPoolUsage as u64,
                quota_non_paged_pool_usage: pmc.QuotaNonPagedPoolUsage as u64,
                page_file_usage: pmc.PagefileUsage as u64,
                peak_page_file_usage: pmc.PeakPagefileUsage as u64,
                private_usage: pmc.PrivateUsage as u64,
            };
            Some(info)
        } else {
            None
        }
    }
}

#[cfg_attr(tarpaulin, skip)]
fn get_command(handle: HANDLE) -> Option<String> {
    unsafe {
        let mut exe_buf = [0u16; MAX_PATH + 1];
        let h_mod = std::ptr::null_mut();

        let ret = GetModuleBaseNameW(
            handle,
            h_mod as _,
            exe_buf.as_mut_ptr(),
            MAX_PATH as DWORD + 1,
        );

        let mut pos = 0;
        for x in exe_buf.iter() {
            if *x == 0 {
                break;
            }
            pos += 1;
        }

        if ret != 0 {
            Some(String::from_utf16_lossy(&exe_buf[..pos]))
        } else {
            None
        }
    }
}

trait RtlUserProcessParameters {
    fn get_cmdline(&self, handle: HANDLE) -> Result<Vec<u16>, &'static str>;
    fn get_cwd(&self, handle: HANDLE) -> Result<Vec<u16>, &'static str>;
    fn get_environ(&self, handle: HANDLE) -> Result<Vec<u16>, &'static str>;
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
                    let size = get_region_size(handle, ptr as LPVOID)?;
                    get_process_data(handle, ptr as _, size as _)
                }
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

#[allow(clippy::uninit_vec)]
unsafe fn get_process_data(
    handle: HANDLE,
    ptr: LPVOID,
    size: usize,
) -> Result<Vec<u16>, &'static str> {
    let mut buffer: Vec<u16> = Vec::with_capacity(size / 2 + 1);
    buffer.set_len(size / 2);
    if ReadProcessMemory(
        handle,
        ptr as *mut _,
        buffer.as_mut_ptr() as *mut _,
        size,
        std::ptr::null_mut(),
    ) != TRUE
    {
        return Err("Unable to read process data");
    }
    Ok(buffer)
}

unsafe fn get_region_size(handle: HANDLE, ptr: LPVOID) -> Result<usize, &'static str> {
    let mut meminfo = MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();
    if VirtualQueryEx(
        handle,
        ptr,
        meminfo.as_mut_ptr() as *mut _,
        size_of::<MEMORY_BASIC_INFORMATION>(),
    ) == 0
    {
        return Err("Unable to read process memory information");
    }
    let meminfo = meminfo.assume_init();
    Ok((meminfo.RegionSize as isize - ptr.offset_from(meminfo.BaseAddress)) as usize)
}

#[allow(clippy::uninit_vec)]
unsafe fn ph_query_process_variable_size(
    process_handle: HANDLE,
    process_information_class: PROCESSINFOCLASS,
) -> Option<Vec<u16>> {
    let mut return_length = MaybeUninit::<ULONG>::uninit();

    let mut status = NtQueryInformationProcess(
        process_handle,
        process_information_class,
        std::ptr::null_mut(),
        0,
        return_length.as_mut_ptr() as *mut _,
    );

    if status != STATUS_BUFFER_OVERFLOW
        && status != STATUS_BUFFER_TOO_SMALL
        && status != STATUS_INFO_LENGTH_MISMATCH
    {
        return None;
    }

    let mut return_length = return_length.assume_init();
    let buf_len = (return_length as usize) / 2;
    let mut buffer: Vec<u16> = Vec::with_capacity(buf_len + 1);
    buffer.set_len(buf_len);

    status = NtQueryInformationProcess(
        process_handle,
        process_information_class,
        buffer.as_mut_ptr() as *mut _,
        return_length,
        &mut return_length as *mut _,
    );
    if !NT_SUCCESS(status) {
        return None;
    }
    buffer.push(0);
    Some(buffer)
}

unsafe fn get_cmdline_from_buffer(buffer: *const u16) -> Vec<String> {
    // Get argc and argv from the command line
    let mut argc = MaybeUninit::<i32>::uninit();
    let argv_p = winapi::um::shellapi::CommandLineToArgvW(buffer, argc.as_mut_ptr());
    if argv_p.is_null() {
        return Vec::new();
    }
    let argc = argc.assume_init();
    let argv = std::slice::from_raw_parts(argv_p, argc as usize);

    let mut res = Vec::new();
    for arg in argv {
        let len = libc::wcslen(*arg);
        let str_slice = std::slice::from_raw_parts(*arg, len);
        res.push(String::from_utf16_lossy(str_slice));
    }

    winapi::um::winbase::LocalFree(argv_p as *mut _);

    res
}

unsafe fn get_process_params(
    handle: HANDLE,
) -> Result<(Vec<String>, Vec<String>, PathBuf), &'static str> {
    if !cfg!(target_pointer_width = "64") {
        return Err("Non 64 bit targets are not supported");
    }

    // First check if target process is running in wow64 compatibility emulator
    let mut pwow32info = MaybeUninit::<LPVOID>::uninit();
    let result = NtQueryInformationProcess(
        handle,
        ProcessWow64Information,
        pwow32info.as_mut_ptr() as *mut _,
        size_of::<LPVOID>() as u32,
        null_mut(),
    );
    if !NT_SUCCESS(result) {
        return Err("Unable to check WOW64 information about the process");
    }
    let pwow32info = pwow32info.assume_init();

    if pwow32info.is_null() {
        // target is a 64 bit process

        let mut pbasicinfo = MaybeUninit::<PROCESS_BASIC_INFORMATION>::uninit();
        let result = NtQueryInformationProcess(
            handle,
            ProcessBasicInformation,
            pbasicinfo.as_mut_ptr() as *mut _,
            size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            null_mut(),
        );
        if !NT_SUCCESS(result) {
            return Err("Unable to get basic process information");
        }
        let pinfo = pbasicinfo.assume_init();

        let mut peb = MaybeUninit::<PEB>::uninit();
        if ReadProcessMemory(
            handle,
            pinfo.PebBaseAddress as *mut _,
            peb.as_mut_ptr() as *mut _,
            size_of::<PEB>() as SIZE_T,
            std::ptr::null_mut(),
        ) != TRUE
        {
            return Err("Unable to read process PEB");
        }

        let peb = peb.assume_init();

        let mut proc_params = MaybeUninit::<RTL_USER_PROCESS_PARAMETERS>::uninit();
        if ReadProcessMemory(
            handle,
            peb.ProcessParameters as *mut PRTL_USER_PROCESS_PARAMETERS as *mut _,
            proc_params.as_mut_ptr() as *mut _,
            size_of::<RTL_USER_PROCESS_PARAMETERS>() as SIZE_T,
            std::ptr::null_mut(),
        ) != TRUE
        {
            return Err("Unable to read process parameters");
        }

        let proc_params = proc_params.assume_init();
        return Ok((
            get_cmd_line(&proc_params, handle),
            get_proc_env(&proc_params, handle),
            get_cwd(&proc_params, handle),
        ));
    }
    // target is a 32 bit process in wow64 mode

    let mut peb32 = MaybeUninit::<PEB32>::uninit();
    if ReadProcessMemory(
        handle,
        pwow32info,
        peb32.as_mut_ptr() as *mut _,
        size_of::<PEB32>() as SIZE_T,
        std::ptr::null_mut(),
    ) != TRUE
    {
        return Err("Unable to read PEB32");
    }
    let peb32 = peb32.assume_init();

    let mut proc_params = MaybeUninit::<RTL_USER_PROCESS_PARAMETERS32>::uninit();
    if ReadProcessMemory(
        handle,
        peb32.ProcessParameters as *mut PRTL_USER_PROCESS_PARAMETERS32 as *mut _,
        proc_params.as_mut_ptr() as *mut _,
        size_of::<RTL_USER_PROCESS_PARAMETERS32>() as SIZE_T,
        std::ptr::null_mut(),
    ) != TRUE
    {
        return Err("Unable to read 32 bit process parameters");
    }
    let proc_params = proc_params.assume_init();
    Ok((
        get_cmd_line(&proc_params, handle),
        get_proc_env(&proc_params, handle),
        get_cwd(&proc_params, handle),
    ))
}

static WINDOWS_8_1_OR_NEWER: Lazy<bool> = Lazy::new(|| {
    let mut version_info: RTL_OSVERSIONINFOEXW = unsafe { MaybeUninit::zeroed().assume_init() };

    version_info.dwOSVersionInfoSize = std::mem::size_of::<RTL_OSVERSIONINFOEXW>() as u32;
    if !NT_SUCCESS(unsafe {
        RtlGetVersion(&mut version_info as *mut RTL_OSVERSIONINFOEXW as *mut _)
    }) {
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

#[allow(clippy::cast_ptr_alignment)]
fn get_cmd_line_new(handle: HANDLE) -> Vec<String> {
    unsafe {
        if let Some(buffer) = ph_query_process_variable_size(handle, ProcessCommandLineInformation)
        {
            let buffer = (*(buffer.as_ptr() as *const UNICODE_STRING)).Buffer;

            get_cmdline_from_buffer(buffer)
        } else {
            vec![]
        }
    }
}

fn get_cmd_line_old<T: RtlUserProcessParameters>(params: &T, handle: HANDLE) -> Vec<String> {
    match params.get_cmdline(handle) {
        Ok(buffer) => unsafe { get_cmdline_from_buffer(buffer.as_ptr()) },
        Err(_e) => {
            // sysinfo_debug!("get_cmd_line_old failed to get data: {}", _e);
            Vec::new()
        }
    }
}

fn get_proc_env<T: RtlUserProcessParameters>(params: &T, handle: HANDLE) -> Vec<String> {
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
                if raw_env[begin..end].iter().any(|&c| c == equals) {
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
            result
        }
        Err(_e) => {
            // sysinfo_debug!("get_proc_env failed to get data: {}", _e);
            Vec::new()
        }
    }
}

fn get_cwd<T: RtlUserProcessParameters>(params: &T, handle: HANDLE) -> PathBuf {
    match params.get_cwd(handle) {
        Ok(buffer) => unsafe { PathBuf::from(null_terminated_wchar_to_string(buffer.as_slice())) },
        Err(_e) => {
            // sysinfo_debug!("get_cwd failed to get data: {}", _e);
            PathBuf::new()
        }
    }
}

#[cfg_attr(tarpaulin, skip)]
fn get_io(handle: HANDLE) -> Option<(u64, u64)> {
    unsafe {
        let mut io: IO_COUNTERS = zeroed();
        let ret = GetProcessIoCounters(handle, &mut io);

        if ret != 0 {
            Some((io.ReadTransferCount, io.WriteTransferCount))
        } else {
            None
        }
    }
}

pub struct SidName {
    pub sid: Vec<u64>,
    pub name: Option<String>,
    pub domainname: Option<String>,
}

#[cfg_attr(tarpaulin, skip)]
fn get_user(handle: HANDLE) -> Option<SidName> {
    unsafe {
        let mut token: HANDLE = zeroed();
        let ret = OpenProcessToken(handle, TOKEN_QUERY, &mut token);

        if ret == 0 {
            return None;
        }

        let mut cb_needed = 0;
        let _ = GetTokenInformation(
            token,
            TokenUser,
            ptr::null::<c_void>() as *mut c_void,
            0,
            &mut cb_needed,
        );

        let mut buf: Vec<u8> = Vec::with_capacity(cb_needed as usize);

        let ret = GetTokenInformation(
            token,
            TokenUser,
            buf.as_mut_ptr() as *mut c_void,
            cb_needed,
            &mut cb_needed,
        );
        buf.set_len(cb_needed as usize);

        if ret == 0 {
            return None;
        }

        #[allow(clippy::cast_ptr_alignment)]
        let token_user = buf.as_ptr() as *const TOKEN_USER;
        let psid = (*token_user).User.Sid;

        let sid = get_sid(psid);
        let (name, domainname) = if let Some((x, y)) = get_name_cached(psid) {
            (Some(x), Some(y))
        } else {
            (None, None)
        };

        Some(SidName {
            sid,
            name,
            domainname,
        })
    }
}

#[cfg_attr(tarpaulin, skip)]
fn get_groups(handle: HANDLE) -> Option<Vec<SidName>> {
    unsafe {
        let mut token: HANDLE = zeroed();
        let ret = OpenProcessToken(handle, TOKEN_QUERY, &mut token);

        if ret == 0 {
            return None;
        }

        let mut cb_needed = 0;
        let _ = GetTokenInformation(
            token,
            TokenGroups,
            ptr::null::<c_void>() as *mut c_void,
            0,
            &mut cb_needed,
        );

        let mut buf: Vec<u8> = Vec::with_capacity(cb_needed as usize);

        let ret = GetTokenInformation(
            token,
            TokenGroups,
            buf.as_mut_ptr() as *mut c_void,
            cb_needed,
            &mut cb_needed,
        );
        buf.set_len(cb_needed as usize);

        if ret == 0 {
            return None;
        }

        #[allow(clippy::cast_ptr_alignment)]
        let token_groups = buf.as_ptr() as *const TOKEN_GROUPS;

        let mut ret = Vec::new();
        let sa = (*token_groups).Groups.as_ptr();
        for i in 0..(*token_groups).GroupCount {
            let psid = (*sa.offset(i as isize)).Sid;
            let sid = get_sid(psid);
            let (name, domainname) = if let Some((x, y)) = get_name_cached(psid) {
                (Some(x), Some(y))
            } else {
                (None, None)
            };

            let sid_name = SidName {
                sid,
                name,
                domainname,
            };
            ret.push(sid_name);
        }

        Some(ret)
    }
}

#[cfg_attr(tarpaulin, skip)]
fn get_sid(psid: PSID) -> Vec<u64> {
    unsafe {
        let mut ret = Vec::new();
        let psid = psid as *const SID;

        let mut ia = 0;
        ia |= u64::from((*psid).IdentifierAuthority.Value[0]) << 40;
        ia |= u64::from((*psid).IdentifierAuthority.Value[1]) << 32;
        ia |= u64::from((*psid).IdentifierAuthority.Value[2]) << 24;
        ia |= u64::from((*psid).IdentifierAuthority.Value[3]) << 16;
        ia |= u64::from((*psid).IdentifierAuthority.Value[4]) << 8;
        ia |= u64::from((*psid).IdentifierAuthority.Value[5]);

        ret.push(u64::from((*psid).Revision));
        ret.push(ia);
        let cnt = (*psid).SubAuthorityCount;
        let sa = (*psid).SubAuthority.as_ptr();
        for i in 0..cnt {
            ret.push(u64::from(*sa.offset(i as isize)));
        }

        ret
    }
}

thread_local!(
    pub static NAME_CACHE: RefCell<HashMap<PSID, Option<(String, String)>>> =
        RefCell::new(HashMap::new());
);

#[cfg_attr(tarpaulin, skip)]
fn get_name_cached(psid: PSID) -> Option<(String, String)> {
    NAME_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if let Some(x) = c.get(&psid) {
            x.clone()
        } else {
            let x = get_name(psid);
            c.insert(psid, x.clone());
            x
        }
    })
}

#[cfg_attr(tarpaulin, skip)]
fn get_name(psid: PSID) -> Option<(String, String)> {
    unsafe {
        let mut cc_name = 0;
        let mut cc_domainname = 0;
        let mut pe_use = 0;
        let _ = LookupAccountSidW(
            ptr::null::<u16>() as *mut u16,
            psid,
            ptr::null::<u16>() as *mut u16,
            &mut cc_name,
            ptr::null::<u16>() as *mut u16,
            &mut cc_domainname,
            &mut pe_use,
        );

        if cc_name == 0 || cc_domainname == 0 {
            return None;
        }

        let mut name: Vec<u16> = Vec::with_capacity(cc_name as usize);
        let mut domainname: Vec<u16> = Vec::with_capacity(cc_domainname as usize);
        name.set_len(cc_name as usize);
        domainname.set_len(cc_domainname as usize);
        let ret = LookupAccountSidW(
            ptr::null::<u16>() as *mut u16,
            psid,
            name.as_mut_ptr() as *mut u16,
            &mut cc_name,
            domainname.as_mut_ptr() as *mut u16,
            &mut cc_domainname,
            &mut pe_use,
        );

        if ret == 0 {
            return None;
        }

        let name = from_wide_ptr(name.as_ptr());
        let domainname = from_wide_ptr(domainname.as_ptr());
        Some((name, domainname))
    }
}

#[cfg_attr(tarpaulin, skip)]
fn from_wide_ptr(ptr: *const u16) -> String {
    // use std::ffi::OsString;
    // use std::os::windows::ffi::OsStringExt;
    unsafe {
        assert!(!ptr.is_null());
        let len = (0..std::isize::MAX)
            .position(|i| *ptr.offset(i) == 0)
            .unwrap_or_default();
        let slice = std::slice::from_raw_parts(ptr, len);
        OsString::from_wide(slice).to_string_lossy().into_owned()
    }
}

#[cfg_attr(tarpaulin, skip)]
fn get_priority(handle: HANDLE) -> u32 {
    unsafe { GetPriorityClass(handle) }
}

impl ProcessInfo {
    /// PID of process
    pub fn pid(&self) -> i32 {
        self.pid
    }

    /// Name of command
    pub fn name(&self) -> String {
        // self.command()
        //     .split(' ')
        //     .collect::<Vec<_>>()
        //     .first()
        //     .map(|x| x.to_string())
        //     .unwrap_or_default()
        self.command.clone()
    }

    /// Full name of command, with arguments
    pub fn command(&self) -> String {
        // self.command.clone()
        self.cmd.join(" ")
    }

    pub fn environ(&self) -> Vec<String> {
        self.environ.clone()
    }

    pub fn cwd(&self) -> String {
        self.cwd.display().to_string()
    }

    /// Get the status of the process
    pub fn status(&self) -> String {
        "unknown".to_string()
    }

    /// CPU usage as a percent of total
    pub fn cpu_usage(&self) -> f64 {
        let curr_time = self.cpu_info.curr_sys + self.cpu_info.curr_user;
        let prev_time = self.cpu_info.prev_sys + self.cpu_info.prev_user;

        let usage_ms = (curr_time - prev_time) / 10000u64;
        let interval_ms = self.interval.as_secs() * 1000 + u64::from(self.interval.subsec_millis());
        usage_ms as f64 * 100.0 / interval_ms as f64
    }

    /// Memory size in number of bytes
    pub fn mem_size(&self) -> u64 {
        self.memory_info.working_set_size
    }

    /// Virtual memory size in bytes
    pub fn virtual_size(&self) -> u64 {
        self.memory_info.private_usage
    }
}
