//! Lightweight walk of parent processes to see if an ancestor is Nushell.
//!
//! Used so a shebang `./foo.nu` can speak NUON when launched from a nu parent
//! without rewriting the spawn into `nu --structured-io foo.nu`.

use std::path::Path;

/// How far to walk when inferring a Nushell parent.
///
/// Covers `nu -> env -> nu` (shebang) and `nu -> cmd.exe -> nu` (Windows PATHEXT).
pub const NUSHELL_ANCESTOR_MAX_DEPTH: u32 = 4;

/// True if some ancestor process (up to `max_depth`) is a Nushell binary.
///
/// Skips `env` / `cmd.exe` wrappers so Windows PATHEXT and `#!/usr/bin/env nu`
/// still resolve to the real parent `nu`.
pub fn ancestor_is_nushell(max_depth: u32) -> bool {
    ancestor_is_nushell_inner(max_depth.max(1))
}

/// Basename looks like the Nushell executable.
pub fn is_nushell_basename(name: &str) -> bool {
    let name = basename_only(name);
    name.eq_ignore_ascii_case("nu") || name.eq_ignore_ascii_case("nu.exe")
}

/// Wrappers that sit between two nu processes and should be skipped.
pub fn is_wrapper_basename(name: &str) -> bool {
    let name = basename_only(name);
    name.eq_ignore_ascii_case("env")
        || name.eq_ignore_ascii_case("env.exe")
        || name.eq_ignore_ascii_case("cmd")
        || name.eq_ignore_ascii_case("cmd.exe")
}

fn basename_only(name: &str) -> &str {
    Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name)
}

#[cfg(not(any(unix, windows)))]
fn ancestor_is_nushell_inner(_max_depth: u32) -> bool {
    false
}

#[cfg(any(unix, windows))]
fn ancestor_is_nushell_inner(max_depth: u32) -> bool {
    let mut pid = current_ppid();
    for _ in 0..max_depth {
        if pid <= 1 {
            return false;
        }
        let Some(name) = process_basename(pid) else {
            return false;
        };
        if is_nushell_basename(&name) {
            return true;
        }
        if !is_wrapper_basename(&name) {
            return false;
        }
        let Some(next) = process_ppid(pid) else {
            return false;
        };
        if next == pid {
            return false;
        }
        pid = next;
    }
    false
}

#[cfg(unix)]
fn current_ppid() -> i32 {
    nix::unistd::getppid().as_raw()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn process_basename(pid: i32) -> Option<String> {
    let path = std::fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_owned())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn process_ppid(pid: i32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_comm = stat.rsplit_once(')')?.1;
    let mut fields = after_comm.split_whitespace();
    let _state = fields.next()?;
    fields.next()?.parse().ok()
}

#[cfg(target_os = "macos")]
fn process_basename(pid: i32) -> Option<String> {
    let path = libproc::libproc::proc_pid::pidpath(pid).ok()?;
    Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_owned())
}

#[cfg(target_os = "macos")]
fn process_ppid(pid: i32) -> Option<i32> {
    let info =
        libproc::libproc::proc_pid::pidinfo::<libproc::libproc::bsd_info::BSDInfo>(pid, 0).ok()?;
    Some(info.pbi_ppid as i32)
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "android", target_os = "macos"))
))]
fn process_basename(pid: i32) -> Option<String> {
    std::fs::read_link(format!("/proc/{pid}/file"))
        .ok()
        .and_then(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_owned())
        })
        .or_else(|| {
            std::fs::read_to_string(format!("/proc/{pid}/comm"))
                .ok()
                .map(|s| s.trim().to_owned())
        })
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "android", target_os = "macos"))
))]
fn process_ppid(pid: i32) -> Option<i32> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("PPid:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

#[cfg(windows)]
fn current_ppid() -> i32 {
    process_ppid(std::process::id() as i32).unwrap_or(0)
}

#[cfg(windows)]
fn process_ppid(pid: i32) -> Option<i32> {
    use std::mem::{size_of, zeroed};
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32, Process32First, Process32Next, TH32CS_SNAPPROCESS,
    };

    // SAFETY: ToolHelp snapshot APIs require a snapshot handle from
    // `CreateToolhelp32Snapshot`. `PROCESSENTRY32` is a Win32 struct whose
    // `dwSize` field must be set before `Process32First`. We close the handle.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry: PROCESSENTRY32 = zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32>() as u32;
        let mut ok = Process32First(snapshot, &mut entry);
        let mut ppid = None;
        while ok.is_ok() {
            if entry.th32ProcessID == pid as u32 {
                ppid = Some(entry.th32ParentProcessID as i32);
                break;
            }
            ok = Process32Next(snapshot, &mut entry);
        }
        let _ = CloseHandle(snapshot);
        ppid
    }
}

#[cfg(windows)]
fn process_basename(pid: i32) -> Option<String> {
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::core::PWSTR;

    // SAFETY: `OpenProcess` fails on a bad pid. The image-name buffer is a
    // writable `PWSTR` whose capacity is passed in `size`. We own and close
    // the process handle.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid as u32).ok()?;
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        result.ok()?;
        let os = std::ffi::OsString::from_wide(&buf[..size as usize]);
        Path::new(&os)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_nu_basename() {
        assert!(is_nushell_basename("nu"));
        assert!(is_nushell_basename("NU.EXE"));
        assert!(is_nushell_basename("/usr/bin/nu"));
        assert!(!is_nushell_basename("nunit"));
        assert!(!is_nushell_basename("bash"));
    }

    #[test]
    fn matches_wrappers() {
        assert!(is_wrapper_basename("env"));
        assert!(is_wrapper_basename("cmd.exe"));
        assert!(!is_wrapper_basename("nu"));
    }

    #[test]
    fn ancestor_walk_does_not_panic() {
        let _ = ancestor_is_nushell(NUSHELL_ANCESTOR_MAX_DEPTH);
    }
}
