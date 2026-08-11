//! Small stable-Rust adapter for handle-relative directory traversal.
//!
//! This intentionally mirrors the direction of the unstable `std::fs::Dir` API tracked by
//! rust-lang/rust#120426. Once `std::fs::Dir` supports stable handle-relative enumeration, this
//! module should become a thin adapter or disappear. Completion keeps its own layer for now because
//! stable `std` cannot yet enumerate an already-open directory handle on all supported platforms.

use std::{
    ffi::{OsStr, OsString},
    io,
    path::Path,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntryKind {
    Directory,
    NonDirectory,
    NeedsLookup,
}

#[derive(Clone, Debug)]
pub(super) struct DirEntry {
    name: OsString,
    kind: EntryKind,
}

impl DirEntry {
    pub(super) fn file_name(&self) -> &OsStr {
        &self.name
    }
}

#[derive(Clone)]
pub(super) struct Dir(platform::Dir);

impl Dir {
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        platform::Dir::open(path).map(Self)
    }

    pub(super) fn entries(&self) -> io::Result<Vec<DirEntry>> {
        self.0.entries()
    }

    /// Opens `name` as a directory relative to this open directory. Extension cost therefore
    /// depends on the child component, not on the depth of the accumulated path.
    pub(super) fn open_dir(&self, name: &OsStr) -> io::Result<Self> {
        self.0.open_dir(name).map(Self)
    }

    /// Opens an enumerated entry as a child directory. Obvious non-directories are rejected
    /// without another syscall; symlinks/reparse points are resolved by the relative open.
    pub(super) fn open_entry_dir(&self, entry: &DirEntry) -> io::Result<Self> {
        if entry.kind == EntryKind::NonDirectory {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "directory entry is not a directory",
            ));
        }
        self.open_dir(entry.file_name())
    }

    pub(super) fn entry_is_dir(&self, entry: &DirEntry) -> bool {
        match entry.kind {
            EntryKind::Directory => true,
            EntryKind::NonDirectory => false,
            EntryKind::NeedsLookup => self.open_entry_dir(entry).is_ok(),
        }
    }
}

#[cfg(unix)]
mod platform {
    use super::{DirEntry, EntryKind};
    use nix::{
        dir::{Dir as NixDir, Type},
        errno::Errno,
        fcntl::{OFlag, open, openat},
        sys::stat::Mode,
    };
    use std::{
        ffi::{OsStr, OsString},
        io,
        os::{fd::OwnedFd, unix::ffi::OsStringExt},
        path::Path,
        sync::{Arc, Mutex},
    };

    #[derive(Clone)]
    pub(super) struct Dir(Arc<DirInner>);

    enum DirInner {
        // A normal readable directory keeps only its fd. Enumeration duplicates this fd into a
        // temporary DIR stream, avoiding both the old openat(fd, ".") pathname lookup and a
        // long-lived libc directory buffer for every ancestor in a deep completion.
        Readable {
            file: std::fs::File,
            enumeration_lock: Mutex<()>,
        },
        // Search-only directories cannot be opened for reading. Keep the O_PATH/O_SEARCH
        // descriptor so exact known-name traversal still works through them.
        TraverseOnly(OwnedFd),
    }

    fn io_error(error: Errno) -> io::Error {
        io::Error::from_raw_os_error(error as i32)
    }

    fn readable_flags() -> OFlag {
        OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn traversal_flags() -> OFlag {
        OFlag::O_PATH | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC
    }

    #[cfg(all(
        unix,
        not(any(target_os = "linux", target_os = "android")),
        any(
            target_vendor = "apple",
            target_os = "solaris",
            target_os = "illumos",
            target_os = "netbsd",
            target_os = "freebsd",
            target_os = "fuchsia",
            target_os = "emscripten",
            target_os = "aix",
            target_os = "wasi"
        )
    ))]
    fn traversal_flags() -> OFlag {
        OFlag::O_SEARCH | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC
    }

    #[cfg(all(
        unix,
        not(any(target_os = "linux", target_os = "android")),
        not(any(
            target_vendor = "apple",
            target_os = "solaris",
            target_os = "illumos",
            target_os = "netbsd",
            target_os = "freebsd",
            target_os = "fuchsia",
            target_os = "emscripten",
            target_os = "aix",
            target_os = "wasi"
        ))
    ))]
    fn traversal_flags() -> OFlag {
        // Some Unix targets expose neither O_PATH nor O_SEARCH. O_RDONLY still
        // provides handle-relative traversal, though it cannot represent a directory
        // that grants search permission without read permission.
        readable_flags()
    }

    impl Dir {
        fn readable(fd: OwnedFd) -> Self {
            Self(Arc::new(DirInner::Readable {
                file: std::fs::File::from(fd),
                enumeration_lock: Mutex::new(()),
            }))
        }

        fn traverse_only(fd: OwnedFd) -> Self {
            Self(Arc::new(DirInner::TraverseOnly(fd)))
        }

        fn should_try_traverse_only(error: Errno) -> bool {
            traversal_flags() != readable_flags() && matches!(error, Errno::EACCES | Errno::EPERM)
        }

        fn open_traverse_only(path: &Path, readable_error: Errno) -> io::Result<Self> {
            if !Self::should_try_traverse_only(readable_error) {
                return Err(io_error(readable_error));
            }
            open(path, traversal_flags(), Mode::empty())
                .map(Self::traverse_only)
                .map_err(io_error)
        }

        pub(super) fn open(path: &Path) -> io::Result<Self> {
            match open(path, readable_flags(), Mode::empty()) {
                Ok(fd) => Ok(Self::readable(fd)),
                Err(error) => Self::open_traverse_only(path, error),
            }
        }

        fn open_dir_from<Fd: std::os::fd::AsFd>(parent: Fd, name: &OsStr) -> io::Result<Self> {
            match openat(&parent, name, readable_flags(), Mode::empty()) {
                Ok(fd) => Ok(Self::readable(fd)),
                Err(error) if Self::should_try_traverse_only(error) => {
                    openat(&parent, name, traversal_flags(), Mode::empty())
                        .map(Self::traverse_only)
                        .map_err(io_error)
                }
                Err(error) => Err(io_error(error)),
            }
        }

        pub(super) fn open_dir(&self, name: &OsStr) -> io::Result<Self> {
            match &*self.0 {
                DirInner::Readable { file, .. } => Self::open_dir_from(file, name),
                DirInner::TraverseOnly(fd) => Self::open_dir_from(fd, name),
            }
        }

        pub(super) fn entries(&self) -> io::Result<Vec<DirEntry>> {
            match &*self.0 {
                DirInner::Readable {
                    file,
                    enumeration_lock,
                } => {
                    // A duplicated descriptor shares the directory offset with `file`, so serialize
                    // enumeration across clones. NixDir::iter rewinds on drop, restoring that shared
                    // offset before the lock is released. This avoids a pathname lookup while keeping
                    // the libc DIR buffer temporary.
                    let _guard = enumeration_lock
                        .lock()
                        .map_err(|_| io::Error::other("directory enumeration lock poisoned"))?;
                    let duplicate: OwnedFd = file.try_clone()?.into();
                    let mut dir = NixDir::from_fd(duplicate).map_err(io_error)?;
                    collect_entries(&mut dir)
                }
                DirInner::TraverseOnly(fd) => {
                    // Permissions may have changed since this handle was opened, so retain the old
                    // behavior of attempting a readable view when enumeration is actually needed.
                    let mut dir = NixDir::openat(fd, ".", readable_flags(), Mode::empty())
                        .map_err(io_error)?;
                    collect_entries(&mut dir)
                }
            }
        }
    }

    fn collect_entries(dir: &mut NixDir) -> io::Result<Vec<DirEntry>> {
        let mut entries = Vec::new();
        for entry in dir.iter() {
            let entry = entry.map_err(io_error)?;
            let name_bytes = entry.file_name().to_bytes();
            if name_bytes == b"." || name_bytes == b".." {
                continue;
            }

            let kind = match entry.file_type() {
                Some(Type::Directory) => EntryKind::Directory,
                Some(Type::Symlink) | None => EntryKind::NeedsLookup,
                Some(_) => EntryKind::NonDirectory,
            };
            entries.push(DirEntry {
                name: OsString::from_vec(name_bytes.to_vec()),
                kind,
            });
        }
        Ok(entries)
    }
}

#[cfg(windows)]
mod platform {
    use super::{DirEntry, EntryKind};
    use std::{
        ffi::{OsStr, OsString},
        io,
        mem::{offset_of, size_of},
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            fs::OpenOptionsExt,
            io::{AsRawHandle, FromRawHandle},
        },
        path::Path,
        ptr, slice,
        sync::{Arc, Mutex},
    };
    use windows_sys::Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FILE_DIRECTORY_FILE, FILE_OPEN, FILE_SYNCHRONOUS_IO_NONALERT, NtCreateFile,
        },
    };
    use windows_sys::Win32::{
        Foundation::{
            ERROR_NO_MORE_FILES, GetLastError, HANDLE, RtlNtStatusToDosError, UNICODE_STRING,
        },
        Storage::FileSystem::{
            FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FULL_DIR_INFO, FILE_LIST_DIRECTORY, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, FILE_TRAVERSE, FileFullDirectoryInfo, FileFullDirectoryRestartInfo,
            GetFileInformationByHandleEx, SYNCHRONIZE,
        },
        System::IO::IO_STATUS_BLOCK,
    };

    const SHARE_ALL: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
    const LIST_ACCESS: u32 = FILE_LIST_DIRECTORY | FILE_TRAVERSE | SYNCHRONIZE;
    const TRAVERSE_ACCESS: u32 = FILE_TRAVERSE | SYNCHRONIZE;

    #[derive(Clone)]
    pub(super) struct Dir {
        file: Arc<std::fs::File>,
        can_list: bool,
        enumeration_lock: Arc<Mutex<()>>,
    }

    impl Dir {
        fn from_file(file: std::fs::File, can_list: bool) -> Self {
            Self {
                file: Arc::new(file),
                can_list,
                enumeration_lock: Arc::new(Mutex::new(())),
            }
        }
    }

    fn ntstatus_error(status: i32) -> io::Error {
        // SAFETY: RtlNtStatusToDosError is a pure conversion routine for an NTSTATUS.
        let error = unsafe { RtlNtStatusToDosError(status) };
        io::Error::from_raw_os_error(error as i32)
    }

    fn unicode_string(name: &OsStr) -> io::Result<(Vec<u16>, UNICODE_STRING)> {
        let mut wide: Vec<u16> = name.encode_wide().collect();
        let byte_len = wide
            .len()
            .checked_mul(size_of::<u16>())
            .filter(|len| *len <= u16::MAX as usize)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "path component too long")
            })?;
        let string = UNICODE_STRING {
            Length: byte_len as u16,
            MaximumLength: byte_len as u16,
            Buffer: wide.as_mut_ptr(),
        };
        Ok((wide, string))
    }

    fn open_root_with_access(path: &Path, access: u32) -> io::Result<std::fs::File> {
        let mut options = std::fs::OpenOptions::new();
        options
            .access_mode(access)
            .share_mode(SHARE_ALL)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS);
        options.open(path)
    }

    impl Dir {
        pub(super) fn open(path: &Path) -> io::Result<Self> {
            match open_root_with_access(path, LIST_ACCESS) {
                Ok(file) => Ok(Self::from_file(file, true)),
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                    open_root_with_access(path, TRAVERSE_ACCESS)
                        .map(|file| Self::from_file(file, false))
                }
                Err(error) => Err(error),
            }
        }

        fn raw_handle(&self) -> HANDLE {
            self.file.as_raw_handle().cast()
        }

        fn open_child_with_access(&self, name: &OsStr, access: u32) -> io::Result<std::fs::File> {
            let (_wide, mut object_name) = unicode_string(name)?;
            let attributes = OBJECT_ATTRIBUTES {
                Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
                RootDirectory: self.raw_handle(),
                ObjectName: &mut object_name,
                // Do not force OBJ_CASE_INSENSITIVE. Completion opens names obtained from
                // enumeration, so preserving native per-directory case sensitivity matters.
                Attributes: 0,
                SecurityDescriptor: ptr::null(),
                SecurityQualityOfService: ptr::null(),
            };
            let mut handle: HANDLE = ptr::null_mut();
            let mut io_status = IO_STATUS_BLOCK::default();

            // SAFETY: all pointers refer to live stack/Vec storage for this synchronous call.
            // On success `handle` is newly owned and immediately transferred to File.
            let status = unsafe {
                NtCreateFile(
                    &mut handle,
                    access,
                    &attributes,
                    &mut io_status,
                    ptr::null(),
                    0,
                    SHARE_ALL,
                    FILE_OPEN,
                    FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT,
                    ptr::null(),
                    0,
                )
            };
            if status < 0 {
                return Err(ntstatus_error(status));
            }

            // SAFETY: NtCreateFile succeeded and transferred ownership of a valid HANDLE.
            Ok(unsafe { std::fs::File::from_raw_handle(handle.cast()) })
        }

        pub(super) fn open_dir(&self, name: &OsStr) -> io::Result<Self> {
            match self.open_child_with_access(name, LIST_ACCESS) {
                Ok(file) => Ok(Self::from_file(file, true)),
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => self
                    .open_child_with_access(name, TRAVERSE_ACCESS)
                    .map(|file| Self::from_file(file, false)),
                Err(error) => Err(error),
            }
        }

        pub(super) fn entries(&self) -> io::Result<Vec<DirEntry>> {
            // GetFileInformationByHandleEx does not itself require FILE_LIST_DIRECTORY.
            // Preserve filesystem access semantics by remembering whether this handle was
            // successfully opened with list rights instead of letting the query API bypass them.
            if !self.can_list {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "directory was opened without list permission",
                ));
            }

            // Directory enumeration has cursor state on the Windows file object. Every call
            // restarts explicitly, and clones of this Dir serialize enumeration so two callers
            // cannot reset each other's cursor mid-scan. Relative child opens do not use this
            // cursor and therefore do not need the lock.
            let _enumeration_guard = self
                .enumeration_lock
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let readable_handle = self.raw_handle();
            const BUFFER_BYTES: usize = 64 * 1024;
            const WORDS: usize = BUFFER_BYTES / size_of::<u64>();
            const NAME_OFFSET: usize = offset_of!(FILE_FULL_DIR_INFO, FileName);

            // FILE_FULL_DIR_INFO requires 8-byte alignment. A u64 allocation provides that.
            let mut storage = vec![0_u64; WORDS];
            let mut entries = Vec::new();
            let mut restart = true;

            loop {
                storage.fill(0);
                let class = if restart {
                    FileFullDirectoryRestartInfo
                } else {
                    FileFullDirectoryInfo
                };
                restart = false;

                // SAFETY: `storage` is writable, 8-byte aligned, and lives for the call.
                // GetFileInformationByHandleEx advances enumeration state on this handle.
                let ok = unsafe {
                    GetFileInformationByHandleEx(
                        readable_handle,
                        class,
                        storage.as_mut_ptr().cast(),
                        BUFFER_BYTES as u32,
                    )
                };
                if ok == 0 {
                    // SAFETY: GetLastError immediately follows the failed Win32 call.
                    let error = unsafe { GetLastError() };
                    if error == ERROR_NO_MORE_FILES {
                        break;
                    }
                    return Err(io::Error::from_raw_os_error(error as i32));
                }

                let base = storage.as_ptr().cast::<u8>();
                let mut offset = 0usize;
                loop {
                    if offset > BUFFER_BYTES.saturating_sub(NAME_OFFSET) {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid directory entry returned by GetFileInformationByHandleEx",
                        ));
                    }

                    // SAFETY: base is 8-byte aligned and every documented NextEntryOffset is
                    // 8-byte aligned; bounds are checked before reading variable data.
                    let info = unsafe { &*base.add(offset).cast::<FILE_FULL_DIR_INFO>() };
                    let name_len = info.FileNameLength as usize;
                    let record_end = offset
                        .checked_add(NAME_OFFSET)
                        .and_then(|end| end.checked_add(name_len));
                    if !name_len.is_multiple_of(size_of::<u16>())
                        || record_end.is_none_or(|end| end > BUFFER_BYTES)
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid filename returned by GetFileInformationByHandleEx",
                        ));
                    }

                    // SAFETY: validation above proves the UTF-16 filename lies in the buffer.
                    let name = unsafe {
                        let ptr = base.add(offset + NAME_OFFSET).cast::<u16>();
                        OsString::from_wide(slice::from_raw_parts(ptr, name_len / size_of::<u16>()))
                    };
                    if name != "." && name != ".." {
                        let attributes = info.FileAttributes;
                        let kind = if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                            EntryKind::NeedsLookup
                        } else if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                            EntryKind::Directory
                        } else {
                            EntryKind::NonDirectory
                        };
                        entries.push(DirEntry { name, kind });
                    }

                    let next = info.NextEntryOffset as usize;
                    if next == 0 {
                        break;
                    }
                    if !next.is_multiple_of(8)
                        || next < NAME_OFFSET
                        || offset
                            .checked_add(next)
                            .is_none_or(|next_offset| next_offset >= BUFFER_BYTES)
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid directory entry offset returned by GetFileInformationByHandleEx",
                        ));
                    }
                    offset += next;
                }
            }

            Ok(entries)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Dir;

    #[test]
    fn entries_restart_from_the_beginning() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let root = temp.path().join("root");
        std::fs::create_dir_all(&root).expect("create root");
        for name in ["a", "b", "c"] {
            std::fs::write(root.join(name), b"fixture").expect("create fixture");
        }

        let dir = Dir::open(&root).expect("open root handle");
        let collect = || {
            let mut names: Vec<_> = dir
                .entries()
                .expect("enumerate directory")
                .into_iter()
                .map(|entry| entry.file_name().to_owned())
                .collect();
            names.sort();
            names
        };

        assert_eq!(collect(), ["a", "b", "c"]);
        assert_eq!(collect(), ["a", "b", "c"]);
    }

    #[test]
    fn handle_relative_traversal_survives_root_rename() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let root = temp.path().join("root");
        let moved = temp.path().join("moved");
        std::fs::create_dir_all(root.join("child/grandchild")).expect("create fixture");

        let handle = Dir::open(&root).expect("open root handle");
        std::fs::rename(&root, &moved).expect("rename open root");
        assert!(!root.exists());

        let names: Vec<_> = handle
            .entries()
            .expect("enumerate renamed directory by handle")
            .into_iter()
            .map(|entry| entry.file_name().to_owned())
            .collect();
        assert!(names.iter().any(|name| name == "child"));

        let child = handle
            .open_dir("child".as_ref())
            .expect("open child relative to renamed root handle");
        assert!(
            child
                .entries()
                .expect("enumerate child")
                .iter()
                .any(|entry| entry.file_name() == "grandchild")
        );
    }
}
