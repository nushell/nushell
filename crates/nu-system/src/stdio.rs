//! Classify stdin/stdout so shebang children can match parent capture.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StdioFd {
    Stdin,
    Stdout,
}

/// True when that standard stream is a pipe (parent pipeline capture), not a TTY or file.
pub fn stdio_is_pipe(fd: StdioFd) -> bool {
    stdio_is_pipe_inner(fd)
}

#[cfg(unix)]
fn stdio_is_pipe_inner(fd: StdioFd) -> bool {
    let raw = match fd {
        StdioFd::Stdin => libc::STDIN_FILENO,
        StdioFd::Stdout => libc::STDOUT_FILENO,
    };
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `fstat` on stdin/stdout with a stack `stat` buffer.
    let rc = unsafe { libc::fstat(raw, stat.as_mut_ptr()) };
    if rc != 0 {
        return false;
    }
    // SAFETY: `fstat` succeeded, so the buffer is initialized.
    let mode = unsafe { stat.assume_init().st_mode };
    let file_type = mode & libc::S_IFMT;
    file_type == libc::S_IFIFO || file_type == libc::S_IFSOCK
}

#[cfg(windows)]
fn stdio_is_pipe_inner(fd: StdioFd) -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{FILE_TYPE_PIPE, GetFileType};

    let raw = match fd {
        StdioFd::Stdin => std::io::stdin().as_raw_handle(),
        StdioFd::Stdout => std::io::stdout().as_raw_handle(),
    };
    // SAFETY: standard handles are valid for the process lifetime.
    unsafe { GetFileType(HANDLE(raw)) == FILE_TYPE_PIPE }
}

#[cfg(not(any(unix, windows)))]
fn stdio_is_pipe_inner(_fd: StdioFd) -> bool {
    false
}
