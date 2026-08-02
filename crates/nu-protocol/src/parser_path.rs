use crate::{
    FileId,
    engine::{StateWorkingSet, VirtualPath},
};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// Maximum size of a script file the `run` command will load for parsing.
///
/// `run` is a parser keyword: interactive re-parse (syntax highlight, completion, validation)
/// reloads the argument path on every keystroke / Tab. Without a bound, a large path is fully
/// loaded and force-fed through the Nu parser, which can hang the REPL and use multi‑GiB of RAM.
///
/// 1 MiB is large enough for typical scripts while keeping interactive parse responsive.
///
/// This limit is intentionally **only** applied to `run`, not `source` / modules.
pub const MAX_RUN_SCRIPT_BYTES: u64 = 1_048_576;

/// Bytes sampled from the start of a file when deciding whether it looks like text.
const TEXT_PROBE_BYTES: usize = 8192;

/// Failure modes when loading a file as a Nu script for `run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptLoadError {
    /// File length exceeds [`MAX_RUN_SCRIPT_BYTES`] (or a custom limit).
    TooLarge { size: u64, max_size: u64 },
    /// Contents do not look like UTF-8 text suitable for a Nu script.
    NotText,
    /// Open/read failed or path is not a readable file.
    Unreadable,
}

/// Heuristic: does `bytes` look like UTF-8 text a Nu script could be?
///
/// Inspects up to the first [`TEXT_PROBE_BYTES`] bytes. Rejects when:
/// - a NUL byte is present (classic binary marker used by git/`file`)
/// - the sample is not valid UTF-8 (Nu source is UTF-8)
/// - more than 30% of bytes are C0 control characters other than tab/LF/CR
///
/// Empty input is treated as text (empty script).
pub fn looks_like_text(bytes: &[u8]) -> bool {
    let sample = if bytes.len() > TEXT_PROBE_BYTES {
        &bytes[..TEXT_PROBE_BYTES]
    } else {
        bytes
    };

    if sample.is_empty() {
        return true;
    }

    // Strong binary signal — archives, executables, compressed data often contain NULs early.
    if sample.contains(&0) {
        return false;
    }

    // Nu scripts are UTF-8; invalid sequences are almost always binary.
    if std::str::from_utf8(sample).is_err() {
        return false;
    }

    // High density of C0 controls (excluding common whitespace) is typical of binary formats
    // that happen to avoid NULs in the first few KiB.
    let suspicious_controls = sample
        .iter()
        .filter(|&&b| b < 0x20 && !matches!(b, b'\t' | b'\n' | b'\r'))
        .count();
    // More than 30% suspicious controls → treat as non-text.
    let mostly_controls = suspicious_controls.saturating_mul(10) > sample.len().saturating_mul(3);
    !mostly_controls
}

/// Read a real filesystem path for `run`, applying size and text checks.
///
/// Size is checked via metadata before any body read so oversized files never enter memory.
pub fn read_run_script_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>, ScriptLoadError> {
    let size = std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(|_| ScriptLoadError::Unreadable)?;
    if size > max_bytes {
        return Err(ScriptLoadError::TooLarge {
            size,
            max_size: max_bytes,
        });
    }
    let contents = std::fs::read(path).map_err(|_| ScriptLoadError::Unreadable)?;
    if !looks_like_text(&contents) {
        return Err(ScriptLoadError::NotText);
    }
    Ok(contents)
}

/// An abstraction over a PathBuf that can have virtual paths (files and directories). Virtual
/// paths always exist and represent a way to ship Nushell code inside the binary without requiring
/// paths to be present in the file system.
///
/// Created from VirtualPath found in the engine state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParserPath {
    RealPath(PathBuf),
    VirtualFile(PathBuf, usize),
    VirtualDir(PathBuf, Vec<ParserPath>),
}

impl ParserPath {
    pub fn is_dir(&self) -> bool {
        match self {
            ParserPath::RealPath(p) => p.is_dir(),
            ParserPath::VirtualFile(..) => false,
            ParserPath::VirtualDir(..) => true,
        }
    }

    pub fn is_file(&self) -> bool {
        match self {
            ParserPath::RealPath(p) => p.is_file(),
            ParserPath::VirtualFile(..) => true,
            ParserPath::VirtualDir(..) => false,
        }
    }

    pub fn exists(&self) -> bool {
        match self {
            ParserPath::RealPath(p) => p.exists(),
            ParserPath::VirtualFile(..) => true,
            ParserPath::VirtualDir(..) => true,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            ParserPath::RealPath(p) => p,
            ParserPath::VirtualFile(p, _) => p,
            ParserPath::VirtualDir(p, _) => p,
        }
    }

    pub fn path_buf(self) -> PathBuf {
        match self {
            ParserPath::RealPath(p) => p,
            ParserPath::VirtualFile(p, _) => p,
            ParserPath::VirtualDir(p, _) => p,
        }
    }

    pub fn parent(&self) -> Option<&Path> {
        match self {
            ParserPath::RealPath(p) => p.parent(),
            ParserPath::VirtualFile(p, _) => p.parent(),
            ParserPath::VirtualDir(p, _) => p.parent(),
        }
    }

    pub fn read_dir(&self) -> Option<Vec<ParserPath>> {
        match self {
            ParserPath::RealPath(p) => p.read_dir().ok().map(|read_dir| {
                read_dir
                    .flatten()
                    .map(|dir_entry| ParserPath::RealPath(dir_entry.path()))
                    .collect()
            }),
            ParserPath::VirtualFile(..) => None,
            ParserPath::VirtualDir(_, files) => Some(files.clone()),
        }
    }

    pub fn file_stem(&self) -> Option<&OsStr> {
        self.path().file_stem()
    }

    pub fn extension(&self) -> Option<&OsStr> {
        self.path().extension()
    }

    pub fn join(self, path: impl AsRef<Path>) -> ParserPath {
        match self {
            ParserPath::RealPath(p) => ParserPath::RealPath(p.join(path)),
            ParserPath::VirtualFile(p, file_id) => ParserPath::VirtualFile(p.join(path), file_id),
            ParserPath::VirtualDir(p, entries) => {
                let new_p = p.join(path);
                let mut pp = ParserPath::RealPath(new_p.clone());
                for entry in entries {
                    if new_p == entry.path() {
                        pp = entry.clone();
                    }
                }
                pp
            }
        }
    }

    pub fn open<'a>(
        &'a self,
        working_set: &'a StateWorkingSet,
    ) -> std::io::Result<Box<dyn std::io::Read + 'a>> {
        match self {
            ParserPath::RealPath(p) => {
                std::fs::File::open(p).map(|f| Box::new(f) as Box<dyn std::io::Read>)
            }
            ParserPath::VirtualFile(_, file_id) => working_set
                .get_contents_of_file(FileId::new(*file_id))
                .map(|bytes| Box::new(bytes) as Box<dyn std::io::Read>)
                .ok_or(std::io::ErrorKind::NotFound.into()),

            ParserPath::VirtualDir(..) => Err(std::io::ErrorKind::NotFound.into()),
        }
    }

    pub fn read<'a>(&'a self, working_set: &'a StateWorkingSet) -> Option<Vec<u8>> {
        self.open(working_set)
            .and_then(|mut reader| {
                let mut vec = vec![];
                reader.read_to_end(&mut vec)?;
                Ok(vec)
            })
            .ok()
    }

    /// File length in bytes when available, without reading the body for real paths.
    pub fn len(&self, working_set: &StateWorkingSet) -> Option<u64> {
        match self {
            ParserPath::RealPath(p) => std::fs::metadata(p).ok().map(|m| m.len()),
            ParserPath::VirtualFile(_, file_id) => working_set
                .get_contents_of_file(FileId::new(*file_id))
                .map(|bytes| bytes.len() as u64),
            ParserPath::VirtualDir(..) => None,
        }
    }

    /// Read file contents for the `run` command, refusing oversized or non-text files.
    ///
    /// Size is checked before reading real paths so interactive parse of `run <path>` cannot hang
    /// on large binaries completed via Tab. Not used by `source` / module loading.
    pub fn read_run_script(
        &self,
        working_set: &StateWorkingSet,
        max_bytes: u64,
    ) -> Result<Vec<u8>, ScriptLoadError> {
        match self {
            ParserPath::RealPath(p) => read_run_script_file(p, max_bytes),
            ParserPath::VirtualFile(_, file_id) => {
                let contents = working_set
                    .get_contents_of_file(FileId::new(*file_id))
                    .ok_or(ScriptLoadError::Unreadable)?;
                let size = contents.len() as u64;
                if size > max_bytes {
                    return Err(ScriptLoadError::TooLarge {
                        size,
                        max_size: max_bytes,
                    });
                }
                if !looks_like_text(contents) {
                    return Err(ScriptLoadError::NotText);
                }
                Ok(contents.to_vec())
            }
            ParserPath::VirtualDir(..) => Err(ScriptLoadError::Unreadable),
        }
    }

    pub fn from_virtual_path(
        working_set: &StateWorkingSet,
        name: &str,
        virtual_path: &VirtualPath,
    ) -> Self {
        match virtual_path {
            VirtualPath::File(file_id) => {
                ParserPath::VirtualFile(PathBuf::from(name), file_id.get())
            }
            VirtualPath::Dir(entries) => ParserPath::VirtualDir(
                PathBuf::from(name),
                entries
                    .iter()
                    .map(|virtual_path_id| {
                        let (virt_name, virt_path) = working_set.get_virtual_path(*virtual_path_id);
                        ParserPath::from_virtual_path(working_set, virt_name, virt_path)
                    })
                    .collect(),
            ),
        }
    }

    /// Normalizes a path to use platform-native separators
    fn normalize_native(path: &str) -> PathBuf {
        Path::new(&path)
            .components()
            .fold(PathBuf::new(), |mut acc, comp| {
                acc.push(comp);
                acc
            })
    }

    /// Normalizes a path to always use forward slashes (good for display, configs, cross-platform strings)
    fn normalize_forward(path: impl AsRef<Path>) -> PathBuf {
        PathBuf::from(
            path.as_ref()
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/"),
        )
    }

    pub fn normalize_slashes_forward(self) -> Self {
        match self {
            ParserPath::RealPath(p) => ParserPath::RealPath(Self::normalize_forward(p)),
            ParserPath::VirtualFile(p, file_id) => {
                ParserPath::VirtualFile(Self::normalize_forward(p), file_id)
            }
            ParserPath::VirtualDir(p, entries) => {
                ParserPath::VirtualDir(Self::normalize_forward(p), entries)
            }
        }
    }

    pub fn normalize_slashes_native(self) -> Self {
        match self {
            ParserPath::RealPath(p) => {
                ParserPath::RealPath(Self::normalize_native(p.to_string_lossy().as_ref()))
            }
            ParserPath::VirtualFile(p, file_id) => ParserPath::VirtualFile(
                Self::normalize_native(p.to_string_lossy().as_ref()),
                file_id,
            ),
            ParserPath::VirtualDir(p, entries) => ParserPath::VirtualDir(
                Self::normalize_native(p.to_string_lossy().as_ref()),
                entries,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_text_accepts_empty_and_normal_scripts() {
        assert!(looks_like_text(b""));
        assert!(looks_like_text(b"def main [] { 'hi' }\n"));
        assert!(looks_like_text(b"let x = 1\t# tab and comment\r\n"));
        // Multi-byte UTF-8 is fine
        assert!(looks_like_text("print '你好'\n".as_bytes()));
    }

    #[test]
    fn looks_like_text_rejects_nul() {
        assert!(!looks_like_text(b"abc\0def"));
    }

    #[test]
    fn looks_like_text_rejects_invalid_utf8() {
        assert!(!looks_like_text(&[0x80, 0x81, 0xFF]));
    }

    #[test]
    fn looks_like_text_rejects_dense_controls() {
        let dense = vec![0x01u8; 50];
        assert!(!looks_like_text(&dense));
    }

    #[test]
    fn looks_like_text_allows_sparse_controls() {
        // A few BEL characters in mostly normal text should still count as text.
        let mut bytes = b"print 'hello'\n".to_vec();
        bytes.push(0x07);
        assert!(looks_like_text(&bytes));
    }
}
