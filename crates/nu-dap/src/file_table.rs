//! File identity: one [`FileId`] per file, however the path was spelled.
//!
//! The same file reaches the adapter spelled differently — absolute from the
//! client's `setBreakpoints`, and relative or tilde'd from the engine, which
//! records `source helper.nu` under exactly that bare name. Breakpoints are
//! set through the first spelling and hit through the second, so the two must
//! compare equal or a breakpoint silently never fires.
//!
//! [`FileTable`] settles that once, on the way in: every spelling is
//! canonicalized and interned to a `FileId`, and everything downstream
//! (breakpoints, valid lines, [`crate::source_map`]) is keyed by the id. There
//! is no second canonicalization anywhere to drift from this one.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A file's identity for the length of a debug session.
///
/// Comparing these instead of path strings is what makes a lookup unable to
/// miss because one side spelled the path differently — and it keeps the
/// per-instruction breakpoint probe off string hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct FileId(u32);

/// The session's path <-> [`FileId`] table.
///
/// Shared, because the two sides of the join run on different threads: the
/// server thread interns the `setBreakpoints` path, the eval thread interns
/// the names in `EngineState::files()`. It is owned by the `Session` and
/// outlives a `restart`, so ids stay valid and carried-over breakpoints keep
/// pointing at the files they were set in.
///
/// Lock discipline: the table's lock is innermost and short-lived — never
/// taken while holding `SessionState`.
#[derive(Clone, Default)]
pub(crate) struct FileTable(Arc<Mutex<FileTableInner>>);

#[derive(Default)]
struct FileTableInner {
    ids: HashMap<String, FileId>,
    paths: Vec<String>,
}

impl FileTable {
    /// Intern a path: the one place a spelling becomes an identity.
    pub(crate) fn intern(&self, path: impl AsRef<Path>) -> FileId {
        let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let key = nu_path::canonicalize_with(path.as_ref(), cwd)
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_else(|_| path.as_ref().to_string_lossy().into_owned());

        let mut inner = self.0.lock().expect("file table poisoned");
        if let Some(&id) = inner.ids.get(&key) {
            return id;
        }
        let id = FileId(inner.paths.len() as u32);
        inner.paths.push(key.clone());
        inner.ids.insert(key, id);
        id
    }

    /// The canonical path `id` was interned from — what the client is handed
    /// as a DAP `Source` and must be able to open. Ids only ever come from
    /// [`Self::intern`], so the fallback is unreachable in practice.
    pub(crate) fn path(&self, id: FileId) -> String {
        let inner = self.0.lock().expect("file table poisoned");
        inner.paths.get(id.0 as usize).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    //! The rules are exercised through `intern`/`path` rather than against
    //! `canonical` directly: the identity it produces is the thing callers
    //! depend on.

    use super::FileTable;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// The reason the table exists: spellings that differ on the wire are one
    /// identity, so a breakpoint set on an absolute path matches the bare name
    /// nu records for a `source`d file.
    #[rstest]
    #[case::bare("Cargo.toml")]
    #[case::dot_prefixed("./Cargo.toml")]
    #[case::through_a_parent("src/../Cargo.toml")]
    fn spellings_of_one_file_intern_to_one_id(#[case] spelling: &str) {
        let files = FileTable::default();
        assert_eq!(files.intern(spelling), files.intern("Cargo.toml"));
    }

    /// Distinct files must not collide, or a breakpoint in one would fire in
    /// the other.
    #[test]
    fn distinct_files_get_distinct_ids() {
        let files = FileTable::default();
        assert_ne!(files.intern("Cargo.toml"), files.intern("src/lib.rs"));
    }

    /// Not every name in `engine_state.files()` is a real path — `<entry-call>`
    /// is synthetic. Such a name still needs a stable identity, and must not be
    /// joined onto the cwd, which would invent a file that never existed.
    #[rstest]
    #[case::synthetic("<entry-call>")]
    #[case::missing_file("does-not-exist-9d1f.nu")]
    fn unresolvable_names_still_intern_stably(#[case] name: &str) {
        let files = FileTable::default();
        let id = files.intern(name);
        assert_eq!(files.intern(name), id);
        assert_eq!(files.path(id), name);
    }

    /// A path round-trips canonicalized but NOT verbatim: `\\?\` paths are what
    /// `std::fs::canonicalize` returns, and they break nu's `source` joining —
    /// which is why `canonical` goes through `nu_path`.
    #[test]
    fn interned_paths_are_absolute_and_not_verbatim() {
        let files = FileTable::default();
        let path = files.path(files.intern("./Cargo.toml"));
        assert!(
            !path.starts_with(r"\\?\"),
            "verbatim prefix survived: {path}"
        );
        assert!(
            std::path::Path::new(&path).is_absolute(),
            "not absolute: {path}"
        );
    }
}
