use std::fs::{self, Metadata};
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// Resolved directory identities on one walk branch. Siblings share the prefix
/// but never mark a directory as visited on each other's branch.
#[derive(Clone, Debug, Default)]
pub(crate) struct Ancestors(Option<Arc<Ancestor>>);

#[derive(Debug)]
struct Ancestor {
    id: DirectoryId,
    depth: usize,
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

/// How a walk tracks the directories above a path, to prune directory cycles.
#[derive(Clone, Debug)]
pub(crate) enum WalkAncestors {
    /// Not inside a recursive walk that follows links, so the walk cannot cycle.
    Untracked,
    /// A recursive walk that has not followed a link yet. Holds the directory
    /// that listed the path.
    BeforeLink(Arc<LazyAncestors>),
    /// Below a followed link. Every directory on the branch has been resolved.
    FollowingLink(Ancestors),
}

/// One directory walked by a `**` component before any link was followed. Its
/// identity is looked up only when a link below it is followed, and at most once
/// per walk, so links under different parents share the lookups they have in common.
#[derive(Debug)]
pub(crate) struct LazyAncestors {
    parent: Option<Arc<Self>>,
    // Directories entered since the walk root.
    depth: usize,
    resolved: OnceLock<Option<Ancestors>>,
}

impl LazyAncestors {
    /// The directory where a `**` walk starts.
    pub(crate) fn root() -> Arc<Self> {
        Arc::new(Self {
            parent: None,
            depth: 0,
            resolved: OnceLock::new(),
        })
    }

    /// A real directory entered from this one.
    pub(crate) fn child(self: &Arc<Self>) -> Arc<Self> {
        Arc::new(Self {
            parent: Some(Arc::clone(self)),
            depth: self.depth + 1,
            resolved: OnceLock::new(),
        })
    }

    /// Resolve the directories between `link` and the walk root, where `self` is
    /// the directory that contains `link`. Returns `None` if they already repeat.
    pub(crate) fn for_link(&self, link: &Path) -> Option<Ancestors> {
        // Climb to the nearest resolved level, then resolve the ones below it in
        // order. A loop rather than recursion keeps deep trees off the stack.
        let mut unresolved = Vec::new();
        let mut ancestors = Some(Ancestors::default());
        let mut level = link.parent().map(|directory| (self, directory));
        while let Some((current, directory)) = level {
            if let Some(resolved) = current.resolved.get() {
                ancestors = resolved.clone();
                break;
            }
            unresolved.push((current, directory));
            level = current.parent.as_deref().zip(directory.parent());
        }
        for (current, directory) in unresolved.into_iter().rev() {
            ancestors = current
                .resolved
                .get_or_init(|| {
                    ancestors.and_then(|ancestors| ancestors.enter_real(directory, current.depth))
                })
                .clone();
        }
        ancestors
    }
}

impl Ancestors {
    /// Enter a directory reached without following a link. One that cannot be
    /// read is skipped; the walker reports its own I/O errors.
    fn enter_real(&self, directory: &Path, depth: usize) -> Option<Self> {
        // A bare relative child has an empty lexical parent, meaning the cwd.
        let directory = if directory.as_os_str().is_empty() {
            Path::new(".")
        } else {
            directory
        };
        match fs::metadata(directory) {
            Ok(metadata) => self.enter_with(directory, &metadata, depth, |_| true),
            Err(_) => Some(self.clone()),
        }
    }

    /// Return the extended branch, or `None` if entering would revisit an ancestor.
    /// The walker still handles any I/O errors from opening the directory.
    pub(crate) fn enter(&self, path: &Path, metadata: &Metadata) -> Option<Self> {
        self.enter_with(path, metadata, 0, |_| true)
    }

    /// For each repeated identity, let the caller decide whether to prune
    /// using the depth at which the ancestor was visited.
    pub(crate) fn enter_with(
        &self,
        path: &Path,
        metadata: &Metadata,
        depth: usize,
        mut prune_revisit: impl FnMut(usize) -> bool,
    ) -> Option<Self> {
        let Some(id) = directory_id(path, metadata) else {
            return Some(self.clone());
        };
        let mut ancestor = self.0.as_deref();
        while let Some(current) = ancestor {
            if current.id == id && prune_revisit(current.depth) {
                return None;
            }
            ancestor = current.parent.0.as_deref();
        }
        Some(Self(Some(Arc::new(Ancestor {
            id,
            depth,
            parent: self.clone(),
        }))))
    }
}
