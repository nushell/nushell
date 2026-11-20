use std::fs::{self, Metadata};
use std::path::Path;
use std::sync::Arc;

/// Resolved directory identities on one walk branch. Siblings share the prefix
/// but never mark a directory as visited on each other's branch.
#[derive(Clone, Debug, Default)]
pub(crate) struct Ancestors(Option<Arc<Ancestor>>);

#[derive(Debug)]
struct Ancestor {
    id: DirectoryId,
    parent: Ancestors,
}

#[cfg(unix)]
type DirectoryId = (u64, u64);
#[cfg(windows)]
type DirectoryId = same_file::Handle;
#[cfg(not(any(unix, windows)))]
type DirectoryId = std::path::PathBuf;

#[cfg(unix)]
fn directory_id(_: &Path, metadata: &Metadata) -> Option<DirectoryId> {
    use std::os::unix::fs::MetadataExt;

    Some((metadata.dev(), metadata.ino()))
}

#[cfg(windows)]
fn directory_id(path: &Path, _: &Metadata) -> Option<DirectoryId> {
    same_file::Handle::from_path(path).ok()
}

#[cfg(not(any(unix, windows)))]
fn directory_id(path: &Path, _: &Metadata) -> Option<DirectoryId> {
    path.canonicalize().ok()
}

impl Ancestors {
    /// Resolve the directories between a followed link and the branch's walk root.
    pub(crate) fn from_parents(link: &Path, count: usize) -> Option<Self> {
        link.ancestors()
            .skip(1)
            .take(count)
            .try_fold(Self::default(), |ancestors, path| {
                // A bare relative child has an empty lexical parent, meaning the cwd.
                let path = if path.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    path
                };
                match fs::metadata(path) {
                    Ok(metadata) => ancestors.enter(path, &metadata),
                    Err(_) => Some(ancestors),
                }
            })
    }

    /// Return the extended branch, or `None` if entering would revisit an ancestor.
    /// The walker still handles any I/O errors from opening the directory.
    pub(crate) fn enter(&self, path: &Path, metadata: &Metadata) -> Option<Self> {
        let Some(id) = directory_id(path, metadata) else {
            return Some(self.clone());
        };
        let mut ancestor = self.0.as_deref();
        while let Some(current) = ancestor {
            if current.id == id {
                return None;
            }
            ancestor = current.parent.0.as_deref();
        }
        Some(Self(Some(Arc::new(Ancestor {
            id,
            parent: self.clone(),
        }))))
    }
}
