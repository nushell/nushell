//! The process listing that every platform's `collect_proc` returns.

use itertools::Itertools;
use std::time::{Duration, SystemTime};

/// The id of the user a process runs as: the numeric uid on unix.
#[cfg(unix)]
pub(crate) type UserId = u32;
/// The id of the user a process runs as: the SID string, such as `S-1-5-18`, on Windows.
#[cfg(windows)]
pub(crate) type UserId = String;

/// A process, with the same details on every platform.
///
/// A detail is `None` when the operating system doesn't make it available for the process (other
/// users' processes, for example), and the details only `ps --long` shows are `None` when
/// `collect_proc` was asked for the short listing. Each platform's `collect_proc` documents where
/// the values come from.
pub struct ProcessInfo {
    pub(crate) pid: i32,
    pub(crate) ppid: i32,
    pub(crate) name: String,
    /// `None` when the command line can't be read, so `command()` shows the name
    pub(crate) command: Option<String>,
    pub(crate) exe: Option<String>,
    pub(crate) user: Option<String>,
    pub(crate) user_id: Option<UserId>,
    pub(crate) status: Option<&'static str>,
    /// Percent of one CPU core over the sample, `None` for a process that started during it
    pub(crate) cpu_usage: Option<f64>,
    pub(crate) cpu_time: Option<Duration>,
    pub(crate) mem_size: Option<u64>,
    pub(crate) virtual_size: Option<u64>,
    pub(crate) private_size: Option<u64>,
    pub(crate) disk_read: Option<u64>,
    pub(crate) disk_written: Option<u64>,
    pub(crate) start_time: Option<SystemTime>,
    pub(crate) process_group_id: Option<i32>,
    pub(crate) session_id: Option<i64>,
    pub(crate) priority: Option<i64>,
    pub(crate) nice: Option<i64>,
    pub(crate) thread_count: Option<i64>,
    pub(crate) cwd: Option<String>,
    pub(crate) environ: Option<Vec<String>>,
}

impl ProcessInfo {
    /// Process id
    pub fn pid(&self) -> i32 {
        self.pid
    }

    /// Parent process id
    pub fn ppid(&self) -> i32 {
        self.ppid
    }

    /// Name of the program
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Command line with arguments, or the name when the command line can't be read
    pub fn command(&self) -> &str {
        self.command.as_deref().unwrap_or(&self.name)
    }

    /// Path of the executable
    pub fn exe(&self) -> Option<&str> {
        self.exe.as_deref()
    }

    /// Name of the user the process runs as
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// Effective user id
    #[cfg(unix)]
    pub fn user_id(&self) -> Option<u32> {
        self.user_id
    }

    /// The SID of the user, such as `S-1-5-18`
    #[cfg(windows)]
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// State of the process, such as "Running" or "Sleeping"
    pub fn status(&self) -> Option<&'static str> {
        self.status
    }

    /// CPU usage as a percent of one core over the sampling interval
    pub fn cpu_usage(&self) -> Option<f64> {
        self.cpu_usage
    }

    /// Total CPU time (user + system) used since the process started
    pub fn cpu_time(&self) -> Option<Duration> {
        self.cpu_time
    }

    /// Resident memory in bytes
    pub fn mem_size(&self) -> Option<u64> {
        self.mem_size
    }

    /// Size of the virtual address space in bytes
    pub fn virtual_size(&self) -> Option<u64> {
        self.virtual_size
    }

    /// Memory only this process uses, in bytes
    pub fn private_size(&self) -> Option<u64> {
        self.private_size
    }

    /// Bytes read since the process started
    pub fn disk_read(&self) -> Option<u64> {
        self.disk_read
    }

    /// Bytes written since the process started
    pub fn disk_written(&self) -> Option<u64> {
        self.disk_written
    }

    /// When the process started
    pub fn start_time(&self) -> Option<SystemTime> {
        self.start_time
    }

    /// Process group id
    pub fn process_group_id(&self) -> Option<i32> {
        self.process_group_id
    }

    /// Session id
    pub fn session_id(&self) -> Option<i64> {
        self.session_id
    }

    /// Scheduling priority
    pub fn priority(&self) -> Option<i64> {
        self.priority
    }

    /// Nice value
    pub fn nice(&self) -> Option<i64> {
        self.nice
    }

    /// Number of threads
    pub fn thread_count(&self) -> Option<i64> {
        self.thread_count
    }

    /// Current working directory
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Environment variables as `KEY=value` strings
    pub fn environ(&self) -> Option<Vec<String>> {
        self.environ.clone()
    }
}

/// Joins a process's arguments into the command line `ps` shows, with newlines and tabs turned
/// into spaces so that each process stays on one line. `None` when there are no arguments.
pub(crate) fn command_line<S: AsRef<str>>(args: &[S]) -> Option<String> {
    (!args.is_empty()).then(|| {
        args.iter()
            .map(AsRef::as_ref)
            .join(" ")
            .replace(['\n', '\t'], " ")
    })
}

/// Names a process the way the BSDs' `ps` does: by its first argument, or when the arguments
/// can't be read (a zombie's or a system process's), by the kernel's name for it, `comm`. The
/// command is the arguments, which `ProcessInfo::command` replaces with the name when there are
/// none. `argv` is the arguments as the kernel reports them, each one ending with a NUL.
#[cfg(any(
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    test
))]
pub(crate) fn name_and_command(argv: &[u8], comm: String) -> (String, Option<String>) {
    let args: Vec<_> = match argv.iter().rposition(|b| *b == 0) {
        Some(end) => argv[..end]
            .split(|b| *b == 0)
            .map(String::from_utf8_lossy)
            .collect(),
        None => Vec::new(),
    };
    let name = match args.first() {
        Some(argv0) if !argv0.is_empty() => argv0.to_string(),
        _ => comm,
    };
    (name, command_line(&args))
}

#[cfg(test)]
mod tests {
    use super::{command_line, name_and_command};

    #[test]
    fn command_line_keeps_each_process_on_one_line() {
        assert_eq!(command_line::<&str>(&[]), None);
        assert_eq!(
            command_line(&["printf", "a\nb", "c\td"]).as_deref(),
            Some("printf a b c d")
        );
    }

    #[test]
    fn bsd_names_come_from_argv0_then_comm() {
        let (name, command) = name_and_command(b"/bin/sh\0-c\0\0echo hi\0", "sh".into());
        assert_eq!(name, "/bin/sh");
        // An empty argument still takes its place.
        assert_eq!(command.as_deref(), Some("/bin/sh -c  echo hi"));

        // A zombie's or system process's arguments can't be read.
        let (name, command) = name_and_command(b"", "zombie".into());
        assert_eq!((name.as_str(), command), ("zombie", None));

        // A process that cleared its argv[0]
        let (name, _) = name_and_command(b"\0arg\0", "kept".into());
        assert_eq!(name, "kept");
    }
}
