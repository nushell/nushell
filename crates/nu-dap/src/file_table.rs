//! File identity: one [`FileId`] per file, however the path was spelled.
//!
//! The same file arrives spelled differently — absolute from the client's
//! `setBreakpoints`, bare from the engine, which records `source helper.nu`
//! under exactly that name. A breakpoint set through one spelling and hit
//! through the other must still match, so [`FileTable`] canonicalizes and
//! interns every spelling on the way in, and everything downstream
//! (breakpoints, valid lines, [`crate::source_map`]) is keyed by the id.
//! Canonicalization happens here and nowhere else, so nothing can drift.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// A file's identity for the length of a debug session. Comparing ids instead
/// of path strings is what makes lookups spelling-proof, and keeps the
/// per-instruction breakpoint probe off string hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct FileId(u32);

/// The session's path <-> [`FileId`] table.
///
/// Shared because the two sides of the join run on different threads: the
/// server thread interns the `setBreakpoints` path, the eval thread the names
/// in `EngineState::files()`. Owned by the `Session` and outliving `restart`,
/// so carried-over breakpoints keep pointing at the files they were set in.
///
/// Lock discipline: innermost and short-lived — never taken while holding
/// `SessionState`.
#[derive(Clone, Default)]
pub(crate) struct FileTable(Arc<Mutex<FileTableInner>>);

#[derive(Default)]
struct FileTableInner {
    ids: HashMap<String, FileId>,
    paths: Vec<String>,
}

impl FileTable {
    /// Intern a path: the one place a spelling becomes an identity.
    ///
    /// Relative spellings resolve against the *live* process cwd, which
    /// `engine::Target::enter_cwd` moves to the script's directory before a
    /// run. That is deliberate: the relative names nu records for `source`d
    /// files are relative to exactly that directory, so interning them there
    /// is what lets them meet the client's absolute path on one id. The
    /// consequence to keep in mind is that the same *relative* spelling
    /// interned before the run and during it can land on different ids;
    /// clients send absolute paths, so it does not bite in practice.
    pub(crate) fn intern(&self, path: impl AsRef<Path>) -> FileId {
        let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let key = nu_path::canonicalize_with(path.as_ref(), cwd)
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_else(|_| path.as_ref().to_string_lossy().into_owned());

        let mut inner = self.0.lock();
        if let Some(&id) = inner.ids.get(&key) {
            return id;
        }
        let id = FileId(inner.paths.len() as u32);
        inner.paths.push(key.clone());
        inner.ids.insert(key, id);
        id
    }

    /// The canonical path `id` was interned from — what the client is handed
    /// as a DAP `Source` and must be able to open. Ids only come from
    /// [`Self::intern`], so the fallback is unreachable.
    pub(crate) fn path(&self, id: FileId) -> String {
        let inner = self.0.lock();
        inner.paths.get(id.0 as usize).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    //! Exercised through `intern`/`path` rather than `canonical` directly:
    //! the identity is what callers depend on.

    use super::FileTable;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// The reason the table exists: a breakpoint set on an absolute path must
    /// match the bare name nu records for a `source`d file.
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

    /// Synthetic names like `<entry-call>` still need a stable identity, and
    /// must not be joined onto the cwd — that would invent a file.
    #[rstest]
    #[case::synthetic("<entry-call>")]
    #[case::missing_file("does-not-exist-9d1f.nu")]
    fn unresolvable_names_still_intern_stably(#[case] name: &str) {
        let files = FileTable::default();
        let id = files.intern(name);
        assert_eq!(files.intern(name), id);
        assert_eq!(files.path(id), name);
    }

    /// Paths round-trip canonicalized but not verbatim: the `\\?\` prefix
    /// `std::fs::canonicalize` returns breaks nu's `source` joining, which is
    /// why interning goes through `nu_path`.
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
