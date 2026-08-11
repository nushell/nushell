use super::{MatchAlgorithm, completion_options::NuMatcher, directory_handle::Dir};
use crate::completions::{
    CompletionOptions, Context, Fetched, SemanticSuggestion, to_reedline_span,
};
use nu_ansi_term::Style;
use nu_engine::env_to_string;
use nu_path::dots::expand_ndots;
use nu_path::{expand_to_real_path, home_dir};
use nu_protocol::{
    Span, SuggestionKind,
    engine::{EngineState, Stack, StateWorkingSet},
};
use nu_utils::IgnoreCaseExt;
use nu_utils::get_ls_colors;
use reedline::Suggestion;
use std::path::{Component, MAIN_SEPARATOR as SEP, Path, PathBuf, is_separator};
use std::{ffi::OsStr, fmt::Write, num::NonZeroUsize};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Default)]
pub struct PathBuiltFromString {
    parts: Vec<MatchedPart>,
    isdir: bool,
}

#[derive(Clone, Default)]
pub struct MatchedPart {
    text: String,
    match_indices: Vec<usize>,
}

/// Arena of output-path components accumulated during one completion request.
///
/// Each traversal candidate stores only a one-word `PathTail` identifying its last component.
/// Extending a path appends one node to this contiguous Vec, so it is O(1) in path depth
/// without a separate node allocation or atomic reference-count update per component.
#[derive(Clone, Copy, Default)]
struct PathTail(Option<NonZeroUsize>);

impl PathTail {
    fn from_index(index: usize) -> Self {
        let encoded = index
            .checked_add(1)
            .and_then(NonZeroUsize::new)
            .expect("path arena index overflow");
        Self(Some(encoded))
    }

    fn index(self) -> Option<usize> {
        self.0.map(|encoded| encoded.get() - 1)
    }
}

#[derive(Default)]
struct PathArena {
    nodes: Vec<PathPartNode>,
}

struct PathPartNode {
    parent: PathTail,
    part: MatchedPart,
}

impl PathArena {
    fn push(&mut self, parent: PathTail, part: MatchedPart) -> PathTail {
        let tail = PathTail::from_index(self.nodes.len());
        self.nodes.push(PathPartNode { parent, part });
        tail
    }

    fn to_vec(&self, tail: PathTail) -> Vec<MatchedPart> {
        // Count first so materialization needs one Vec allocation rather than collecting
        // references and then allocating a second Vec for the cloned output parts.
        let mut len = 0;
        let mut current = tail;
        while let Some(index) = current.index() {
            len += 1;
            current = self.nodes[index].parent;
        }

        let mut parts = Vec::with_capacity(len);
        let mut current = tail;
        while let Some(index) = current.index() {
            let node = &self.nodes[index];
            parts.push(node.part.clone());
            current = node.parent;
        }
        parts.reverse();
        parts
    }
}

/// Index of an open directory ancestry node used only for Windows lexical `..` traversal.
#[cfg(windows)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DirTail(usize);

#[cfg(windows)]
#[derive(Default)]
struct DirArena {
    nodes: Vec<DirNode>,
}

#[cfg(windows)]
enum DirParent {
    Known(DirTail),
    Lexical(PathBuf),
}

#[cfg(windows)]
struct DirNode {
    dir: Dir,
    parent: DirParent,
}

#[cfg(windows)]
impl DirArena {
    fn get(&self, tail: DirTail) -> &Dir {
        &self.nodes[tail.0].dir
    }

    fn push_root(&mut self, dir: Dir, path: &Path) -> DirTail {
        let tail = DirTail(self.nodes.len());
        // Cache the filesystem-root self edge immediately. Repeated `..` at a root then needs
        // neither a pathname scan nor a syscall.
        let parent = if path.has_root() && path.parent().is_none() {
            DirParent::Known(tail)
        } else {
            DirParent::Lexical(path.to_path_buf())
        };
        self.nodes.push(DirNode { dir, parent });
        tail
    }

    fn push_relative(&mut self, parent: DirTail, dir: Dir) -> DirTail {
        debug_assert!(parent.0 < self.nodes.len());
        let tail = DirTail(self.nodes.len());
        self.nodes.push(DirNode {
            dir,
            parent: DirParent::Known(parent),
        });
        tail
    }

    fn mark(&self) -> usize {
        self.nodes.len()
    }

    fn truncate(&mut self, mark: usize) {
        debug_assert!(mark <= self.nodes.len());
        self.nodes.truncate(mark);
    }

    fn lexical_parent(&mut self, tail: DirTail) -> std::io::Result<DirTail> {
        let parent_path = match &self.nodes[tail.0].parent {
            DirParent::Known(parent) => return Ok(*parent),
            DirParent::Lexical(path) => match path.parent() {
                Some(parent) if parent.as_os_str().is_empty() => PathBuf::from("."),
                Some(parent) => parent.to_path_buf(),
                None => path.join(".."),
            },
        };

        // This is the only Windows `..` case that needs a pathname open. The caller scopes
        // nodes created while following this branch, so dead branches release their handles.
        let dir = Dir::open(&parent_path)?;
        Ok(self.push_root(dir, &parent_path))
    }
}

#[derive(Default)]
struct TraversalArenas {
    paths: PathArena,
    #[cfg(windows)]
    dirs: DirArena,
}

/// A live traversal branch. Unix keeps its directory handle directly on the hot path. Windows
/// stores only a one-word ancestry index, so returning through an already traversed component is
/// an O(1) arena lookup with no syscall, pathname reconstruction, or refcount update.
#[derive(Clone)]
#[cfg_attr(windows, derive(Copy))]
struct TraversalCandidate {
    #[cfg(windows)]
    dir: DirTail,
    #[cfg(not(windows))]
    dir: Dir,
    tail: PathTail,
}

impl TraversalArenas {
    fn root_candidate(&mut self, dir: Dir, path: &Path) -> TraversalCandidate {
        #[cfg(windows)]
        let dir = self.dirs.push_root(dir, path);
        #[cfg(not(windows))]
        let _ = path;
        TraversalCandidate {
            dir,
            tail: PathTail::default(),
        }
    }

    fn child_candidate(
        &mut self,
        parent: &TraversalCandidate,
        dir: Dir,
        tail: PathTail,
    ) -> TraversalCandidate {
        #[cfg(windows)]
        let dir = self.dirs.push_relative(parent.dir, dir);
        #[cfg(not(windows))]
        let _ = parent;
        TraversalCandidate { dir, tail }
    }

    fn dir<'a>(&'a self, candidate: &'a TraversalCandidate) -> &'a Dir {
        #[cfg(windows)]
        {
            self.dirs.get(candidate.dir)
        }
        #[cfg(not(windows))]
        {
            &candidate.dir
        }
    }

    #[inline]
    fn scoped_dirs<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        #[cfg(windows)]
        let mark = self.dirs.mark();
        let result = f(self);
        #[cfg(windows)]
        self.dirs.truncate(mark);
        result
    }
}

#[derive(Clone, Copy)]
struct TraversalSettings {
    want_directory: bool,
}

/// Recursively goes through paths that match a given `partial`.
/// * `built_paths`: Open-directory state for valid matching paths built so far.
/// * `want_directory`: Whether final completion matches must be directories.
/// * `isdir`: whether the current partial path has a trailing slash.
///   Parsing a path string into a pathbuf loses that bit of information.
/// * `enable_exact_match`: Whether match algorithm is Prefix and all previous components
///   of the path matched a directory exactly.
fn complete_rec(
    partial: &[&str],
    built_paths: &[TraversalCandidate],
    arenas: &mut TraversalArenas,
    settings: &TraversalSettings,
    options: &CompletionOptions,
    isdir: bool,
    enable_exact_match: bool,
) -> Vec<PathBuiltFromString> {
    let has_more = !partial.is_empty() && (partial.len() > 1 || isdir);

    if let Some((&base, rest)) = partial.split_first()
        && base.chars().all(|c| c == '.')
        && has_more
    {
        return arenas.scoped_dirs(|arenas| {
            let built_paths: Vec<_> = built_paths
                .iter()
                .filter_map(|built| {
                    let tail = arenas.paths.push(
                        built.tail,
                        MatchedPart {
                            text: base.to_string(),
                            match_indices: Vec::new(),
                        },
                    );
                    if base == "." {
                        #[cfg(windows)]
                        let mut candidate = *built;
                        #[cfg(not(windows))]
                        let mut candidate = built.clone();
                        candidate.tail = tail;
                        Some(candidate)
                    } else {
                        #[cfg(windows)]
                        {
                            let dir = arenas.dirs.lexical_parent(built.dir).ok()?;
                            Some(TraversalCandidate { dir, tail })
                        }
                        #[cfg(not(windows))]
                        {
                            // Unix handle-relative `..` preserves native physical traversal semantics,
                            // including after following a symlink.
                            let dir = built.dir.open_dir(OsStr::new(base)).ok()?;
                            Some(TraversalCandidate { dir, tail })
                        }
                    }
                })
                .collect();
            complete_rec(
                rest,
                &built_paths,
                arenas,
                settings,
                options,
                isdir,
                enable_exact_match,
            )
        });
    }

    if has_more {
        complete_intermediate(
            partial,
            built_paths,
            arenas,
            settings,
            options,
            isdir,
            enable_exact_match,
        )
    } else {
        complete_final(
            partial.first().unwrap_or(&""),
            built_paths,
            arenas,
            options,
            settings.want_directory,
        )
    }
}

fn complete_intermediate(
    partial: &[&str],
    built_paths: &[TraversalCandidate],
    arenas: &mut TraversalArenas,
    settings: &TraversalSettings,
    options: &CompletionOptions,
    isdir: bool,
    enable_exact_match: bool,
) -> Vec<PathBuiltFromString> {
    let prefix = partial.first().unwrap_or(&"");
    let prefix_os = OsStr::new(prefix);
    let mut matcher = NuMatcher::new(prefix, options, true);
    let mut completions = Vec::new();

    for built in built_paths {
        let entries = match arenas.dir(built).entries() {
            Ok(entries) => entries,
            Err(err) => {
                // Enumeration needs read/list permission, while handle-relative lookup
                // of a known child may need only search/traverse permission. If listing
                // is denied, there are no sibling names to discover: follow only the
                // literal component the user supplied. Preserve exactness so a branch
                // that became non-exact earlier never regains exact pruning.
                if !prefix.is_empty()
                    && err.kind() == std::io::ErrorKind::PermissionDenied
                    && let Ok(dir) = arenas.dir(built).open_dir(prefix_os)
                {
                    let tail = arenas.paths.push(
                        built.tail,
                        MatchedPart {
                            text: prefix.to_string(),
                            match_indices: (0..prefix.graphemes(true).count()).collect(),
                        },
                    );
                    let branch = arenas.scoped_dirs(|arenas| {
                        let literal = arenas.child_candidate(built, dir, tail);
                        complete_rec(
                            &partial[1..],
                            &[literal],
                            arenas,
                            settings,
                            options,
                            isdir,
                            enable_exact_match,
                        )
                    });
                    completions.extend(branch);
                }
                continue;
            }
        };

        if enable_exact_match {
            // Look only for exact-name uniqueness first. Do not allocate/build ordinary
            // matcher candidates unless this shortcut fails. Opening an exact directory
            // also produces the child handle needed by the next recursion in the same
            // operation; obvious non-directory entries are rejected from their type hint.
            let mut exact = None;
            let mut multiple_exact_matches = false;

            for entry in &entries {
                let entry_name = entry.file_name().to_string_lossy();
                let is_exact = if options.case_sensitive {
                    entry_name.as_ref() == *prefix
                } else {
                    entry_name.eq_ignore_case(prefix)
                };
                if !is_exact {
                    continue;
                }

                let Ok(dir) = arenas.dir(built).open_entry_dir(entry) else {
                    continue;
                };
                if exact.is_none() {
                    exact = Some((entry_name.into_owned(), dir));
                } else {
                    multiple_exact_matches = true;
                    break;
                }
            }

            if !multiple_exact_matches && let Some((entry_name, dir)) = exact {
                let tail = arenas.paths.push(
                    built.tail,
                    MatchedPart {
                        match_indices: (0..entry_name.graphemes(true).count()).collect(),
                        text: entry_name,
                    },
                );
                let branch = arenas.scoped_dirs(|arenas| {
                    let exact = arenas.child_candidate(built, dir, tail);
                    complete_rec(
                        &partial[1..],
                        &[exact],
                        arenas,
                        settings,
                        options,
                        isdir,
                        true,
                    )
                });
                completions.extend(branch);
                continue;
            }
        }

        for entry in entries {
            let entry_name = entry.file_name().to_string_lossy();
            let Some(prepared) = matcher.prepare_match(entry_name.as_ref()) else {
                continue;
            };

            // Keep names borrowed until they actually match. This avoids allocating a String for
            // every non-matching directory entry, which matters in wide directories. Committing
            // the prepared match also avoids running the matcher a second time for survivors.
            // Borrow the parent handle through sorting instead of cloning its refcount once per
            // matching sibling. Child handles are still opened one-at-a-time before recursion.
            matcher.add_prepared_owned(
                entry_name.into_owned(),
                prepared,
                (built.tail, built, entry),
            );
        }
    }

    for ((tail, parent, entry), match_indices) in matcher.results() {
        // A directory-type hint rejects obvious files without a syscall. A surviving
        // entry is opened relative to its parent exactly once to obtain the next handle.
        let Ok(dir) = arenas.dir(parent).open_entry_dir(&entry) else {
            continue;
        };
        let entry_name = entry.file_name().to_string_lossy().into_owned();
        let tail = arenas.paths.push(
            tail,
            MatchedPart {
                text: entry_name,
                match_indices,
            },
        );
        let branch = arenas.scoped_dirs(|arenas| {
            let candidate = arenas.child_candidate(parent, dir, tail);
            complete_rec(
                &partial[1..],
                &[candidate],
                arenas,
                settings,
                options,
                isdir,
                false,
            )
        });
        completions.extend(branch);
    }
    completions
}

fn complete_final(
    prefix: &str,
    built_paths: &[TraversalCandidate],
    arenas: &TraversalArenas,
    options: &CompletionOptions,
    want_directory: bool,
) -> Vec<PathBuiltFromString> {
    let mut matcher = NuMatcher::new(prefix, options, true);

    for built in built_paths {
        let dir = arenas.dir(built);
        let Ok(entries) = dir.entries() else {
            continue;
        };

        for entry in entries {
            let entry_name = entry.file_name().to_string_lossy();
            let Some(prepared) = matcher.prepare_match(entry_name.as_ref()) else {
                continue;
            };

            let entry_isdir = dir.entry_is_dir(&entry);
            if want_directory && !entry_isdir {
                continue;
            }
            matcher.add_prepared_owned(
                entry_name.into_owned(),
                prepared,
                (built.tail, entry, entry_isdir),
            );
        }
    }

    matcher
        .results()
        .into_iter()
        .map(|((tail, entry, isdir), match_indices)| {
            let mut parts = arenas.paths.to_vec(tail);
            parts.push(MatchedPart {
                text: entry.file_name().to_string_lossy().into_owned(),
                match_indices,
            });
            PathBuiltFromString { parts, isdir }
        })
        .collect()
}

#[cfg(test)]
mod path_traversal_tests {
    use super::*;

    #[test]
    fn path_tail_is_one_word() {
        assert_eq!(
            std::mem::size_of::<PathTail>(),
            std::mem::size_of::<usize>()
        );
    }

    #[test]
    fn path_arena_materializes_shared_prefixes() {
        let mut arena = PathArena::default();
        let root = arena.push(
            PathTail::default(),
            MatchedPart {
                text: "root".into(),
                match_indices: vec![0],
            },
        );
        let left = arena.push(
            root,
            MatchedPart {
                text: "left".into(),
                match_indices: vec![1],
            },
        );
        let right = arena.push(
            root,
            MatchedPart {
                text: "right".into(),
                match_indices: vec![2],
            },
        );

        let texts = |tail| {
            arena
                .to_vec(tail)
                .into_iter()
                .map(|part| part.text)
                .collect::<Vec<_>>()
        };
        assert_eq!(texts(left), ["root", "left"]);
        assert_eq!(texts(right), ["root", "right"]);
    }

    fn complete_directory_path(roots: &[PathBuf], partial: &[&str]) -> Vec<String> {
        let mut arenas = TraversalArenas::default();
        let built_paths: Vec<_> = roots
            .iter()
            .map(|root| {
                let dir = Dir::open(root).expect("open completion root");
                arenas.root_candidate(dir, root)
            })
            .collect();
        let options = CompletionOptions::default();
        let settings = TraversalSettings {
            want_directory: true,
        };
        let mut results: Vec<_> = complete_rec(
            partial,
            &built_paths,
            &mut arenas,
            &settings,
            &options,
            true,
            options.match_algorithm == MatchAlgorithm::Prefix,
        )
        .into_iter()
        .map(|path| {
            path.parts
                .iter()
                .map(|part| part.text.as_str())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
        results.sort();
        results
    }

    #[test]
    fn exact_pruning_is_independent_for_each_cwd() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let one = temp.path().join("one");
        let two = temp.path().join("two");
        std::fs::create_dir_all(one.join("foo/a")).expect("create first fixture");
        std::fs::create_dir_all(two.join("foobar/b")).expect("create second fixture");

        assert_eq!(
            complete_directory_path(&[one, two], &["foo"]),
            ["foo/a", "foobar/b"]
        );
    }

    #[test]
    fn exact_directory_in_each_cwd_keeps_each_cwd() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let one = temp.path().join("one");
        let two = temp.path().join("two");
        std::fs::create_dir_all(one.join("foo/a")).expect("create first fixture");
        std::fs::create_dir_all(two.join("foo/b")).expect("create second fixture");

        assert_eq!(
            complete_directory_path(&[one, two], &["foo"]),
            ["foo/a", "foo/b"]
        );
    }

    #[test]
    fn later_exact_component_does_not_restore_pruning() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let root = temp.path().join("root");
        std::fs::create_dir_all(root.join("partial/hol/foo")).expect("create exact fixture");
        std::fs::create_dir_all(root.join("partial-a/hola/foo")).expect("create prefix fixture");

        assert_eq!(
            complete_directory_path(&[root], &["part", "hol"]),
            ["partial-a/hola/foo", "partial/hol/foo"]
        );
    }

    #[test]
    fn exact_file_does_not_shadow_prefix_directory() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let root = temp.path().join("root");
        std::fs::create_dir_all(root.join("foobar/child")).expect("create directory fixture");
        std::fs::write(root.join("foo"), b"file").expect("create exact file fixture");

        assert_eq!(complete_directory_path(&[root], &["foo"]), ["foobar/child"]);
    }

    #[test]
    fn case_sensitive_exact_pruning_uses_directory_entry_spelling() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let root = temp.path().join("root");
        std::fs::create_dir_all(root.join("Foo/wrong-case"))
            .expect("create differently-cased fixture");
        std::fs::create_dir_all(root.join("foobar/right-prefix"))
            .expect("create matching prefix fixture");

        assert_eq!(
            complete_directory_path(&[root], &["foo"]),
            ["foobar/right-prefix"]
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_known_parent_reuses_ancestor_handle() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let root_path = temp.path().join("root");
        std::fs::create_dir_all(root_path.join("child")).expect("create fixture");

        let mut arenas = TraversalArenas::default();
        let root =
            arenas.root_candidate(Dir::open(&root_path).expect("open root handle"), &root_path);
        let child_dir = arenas
            .dir(&root)
            .open_dir(OsStr::new("child"))
            .expect("open child handle");
        let child = arenas.child_candidate(&root, child_dir, PathTail::default());
        let before = arenas.dirs.nodes.len();

        assert_eq!(arenas.dirs.lexical_parent(child.dir).unwrap(), root.dir);
        assert_eq!(arenas.dirs.nodes.len(), before);
    }

    #[cfg(windows)]
    #[test]
    fn windows_recursive_branches_release_directory_handles() {
        let temp = tempfile::tempdir().expect("create tempdir");
        let root_path = temp.path().join("root/a/b");
        std::fs::create_dir_all(root_path.join("child/leaf")).expect("create child fixture");

        let mut arenas = TraversalArenas::default();
        let root =
            arenas.root_candidate(Dir::open(&root_path).expect("open root handle"), &root_path);
        let root_nodes = arenas.dirs.nodes.len();
        let options = CompletionOptions::default();
        let settings = TraversalSettings {
            want_directory: true,
        };

        let _ = complete_rec(
            &["child"],
            &[root],
            &mut arenas,
            &settings,
            &options,
            true,
            true,
        );
        assert_eq!(arenas.dirs.nodes.len(), root_nodes);

        let _ = complete_rec(
            &["..", ".."],
            &[root],
            &mut arenas,
            &settings,
            &options,
            true,
            true,
        );
        assert_eq!(arenas.dirs.nodes.len(), root_nodes);
    }
}

#[derive(Debug)]
enum OriginalCwd {
    None,
    Home,
    Prefix(String),
}

pub fn surround_remove(partial: &str) -> String {
    for c in ['`', '"', '\''] {
        if partial.starts_with(c) {
            let ret = partial.strip_prefix(c).unwrap_or(partial);
            return match ret.split(c).collect::<Vec<_>>()[..] {
                [inside] => inside.to_string(),
                [inside, outside] if inside.ends_with(is_separator) => format!("{inside}{outside}"),
                _ => ret.to_string(),
            };
        }
    }
    partial.to_string()
}

pub struct FileSuggestion {
    pub span: nu_protocol::Span,
    pub path: String,
    pub style: Option<Style>,
    pub is_dir: bool,
    pub display_override: Option<String>,
    pub match_indices: Vec<usize>,
}

/// Sort hidden entries (file names starting with `.`) after visible ones; stable, so
/// order within each group is preserved.
fn hidden_files_last(items: &mut [SemanticSuggestion]) {
    items.sort_by_key(|item| {
        Path::new(&item.suggestion.value)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'))
    });
}

/// Path completion shared by [`FileCompletion`](crate::completions::FileCompletion) and
/// [`DirectoryCompletion`](crate::completions::DirectoryCompletion).
///
/// `directories_only` restricts results to directories; file completion also restricts
/// when the view was readjusted mid-path.
pub(crate) fn complete_paths(ctx: &Context, directories_only: bool) -> Fetched {
    let AdjustView {
        prefix,
        span,
        readjusted,
    } = adjust_if_intermediate(ctx.prefix_str().as_ref(), ctx.working_set, ctx.span);

    #[allow(deprecated)]
    let mut items: Vec<_> = complete_item(
        directories_only || readjusted,
        span,
        &prefix,
        &[&ctx.working_set.permanent_state.current_work_dir()],
        ctx.options,
        ctx.working_set.permanent_state,
        ctx.stack,
    )
    .into_iter()
    .map(|x| SemanticSuggestion {
        suggestion: Suggestion {
            value: x.path,
            style: x.style,
            span: to_reedline_span(x.span, ctx.offset),
            display_override: x.display_override,
            match_indices: Some(x.match_indices),
            ..Suggestion::default()
        },
        kind: Some(if x.is_dir {
            SuggestionKind::Directory
        } else {
            SuggestionKind::File
        }),
    })
    .collect();

    hidden_files_last(&mut items);
    Fetched::Cacheable(items)
}

/// # Parameters
/// * `cwds` - A list of directories in which to search. The only reason this isn't a single string
///   is because dotnu_completions searches in multiple directories at once
pub fn complete_item(
    want_directory: bool,
    span: nu_protocol::Span,
    partial: &str,
    cwds: &[impl AsRef<str>],
    options: &CompletionOptions,
    engine_state: &EngineState,
    stack: &Stack,
) -> Vec<FileSuggestion> {
    let cleaned_partial = surround_remove(partial);
    let isdir = cleaned_partial.ends_with(is_separator);
    let expanded_partial = expand_ndots(Path::new(&cleaned_partial));
    let should_collapse_dots = expanded_partial != Path::new(&cleaned_partial);
    let mut partial = expanded_partial.to_string_lossy().to_string();

    // Echo back whichever separator the user last typed; Windows accepts both.
    let path_separator = if cfg!(windows) {
        cleaned_partial
            .chars()
            .rfind(|c: &char| is_separator(*c))
            .unwrap_or(SEP)
    } else {
        SEP
    };

    // Handle the trailing dot case
    if cleaned_partial.ends_with(&format!("{path_separator}.")) {
        let _ = write!(partial, "{path_separator}.");
    }

    let cwd_pathbufs: Vec<_> = cwds
        .iter()
        .map(|cwd| Path::new(cwd.as_ref()).to_path_buf())
        .collect();
    let ls_colors = (engine_state.config.completions.use_ls_colors
        && engine_state.config.use_ansi_coloring.get(engine_state))
    .then(|| {
        let ls_colors_env_str = stack
            .get_env_var(engine_state, "LS_COLORS")
            .and_then(|v| env_to_string("LS_COLORS", v, engine_state, stack).ok());
        get_ls_colors(ls_colors_env_str)
    });

    let mut cwds = cwd_pathbufs.clone();
    let mut prefix_len = 0;
    let mut original_cwd = OriginalCwd::None;

    let mut components = Path::new(&partial).components().peekable();
    match components.peek().cloned() {
        Some(c @ Component::Prefix(..)) => {
            // windows only by definition
            cwds = vec![[c, Component::RootDir].iter().collect()];
            prefix_len = c.as_os_str().len();
            original_cwd = OriginalCwd::Prefix(c.as_os_str().to_string_lossy().into_owned());
        }
        Some(c @ Component::RootDir) => {
            // This is kind of a hack. When joining an empty string with the rest,
            // we add the slash automagically
            cwds = vec![PathBuf::from(c.as_os_str())];
            prefix_len = 1;
            original_cwd = OriginalCwd::Prefix(String::new());
        }
        Some(Component::Normal(home)) if home.to_string_lossy() == "~" => {
            cwds = home_dir().map(|dir| vec![dir.into()]).unwrap_or_default();
            prefix_len = 1;
            original_cwd = OriginalCwd::Home;
        }
        _ => {}
    };

    // `prefix_len` is a byte length from a `Component` that may not land on a char
    // boundary of `partial`, so slice fallibly to avoid a panic on user input.
    let after_prefix = partial.get(prefix_len..).unwrap_or_default();
    let partial: Vec<_> = after_prefix
        .strip_prefix(is_separator)
        .unwrap_or(after_prefix)
        .split(is_separator)
        .filter(|s| !s.is_empty())
        .collect();

    let mut arenas = TraversalArenas::default();
    let built_paths: Vec<_> = cwds
        .iter()
        .filter_map(|cwd| {
            Dir::open(cwd)
                .ok()
                .map(|dir| arenas.root_candidate(dir, cwd))
        })
        .collect();

    let settings = TraversalSettings { want_directory };
    complete_rec(
        partial.as_slice(),
        &built_paths,
        &mut arenas,
        &settings,
        options,
        isdir,
        options.match_algorithm == MatchAlgorithm::Prefix,
    )
    .into_iter()
    .map(|mut p| {
        if should_collapse_dots {
            p = collapse_ndots(p);
        }
        let is_dir = p.isdir;

        let mut path = match &original_cwd {
            OriginalCwd::None => String::new(),
            OriginalCwd::Home => format!("~{path_separator}"),
            OriginalCwd::Prefix(s) => format!("{s}{path_separator}"),
        };
        let mut match_index_offset = path.graphemes(true).count();
        let mut match_indices = Vec::new();
        for (i, part) in p.parts.iter().enumerate() {
            path.push_str(&part.text);
            for ind in &part.match_indices {
                match_indices.push(ind + match_index_offset);
            }
            match_index_offset += part.text.graphemes(true).count();
            if i != p.parts.len() - 1 {
                path.push(path_separator);
                // One grapheme, not `len_utf8()`: this offset counts graphemes.
                match_index_offset += 1;
            }
        }
        if p.isdir {
            path.push(path_separator);
        }

        let real_path = expand_to_real_path(&path);
        let metadata = std::fs::symlink_metadata(&real_path).ok();
        let style = ls_colors.as_ref().map(|lsc| {
            lsc.style_for_path_with_metadata(&real_path, metadata.as_ref())
                .map(lscolors::Style::to_nu_ansi_term_style)
                .unwrap_or_default()
        });
        let (value, display_override) = if let Some(escaped) = escape_path(&path) {
            (escaped, Some(path))
        } else {
            (path, None)
        };
        FileSuggestion {
            span,
            path: value,
            style,
            is_dir,
            display_override,
            match_indices,
        }
    })
    .collect()
}

/// Fix files or folders with quotes or hashes.
/// Returns `None` if nothing had to be escaped.
pub fn escape_path(path: &str) -> Option<String> {
    // make glob pattern have the highest priority.
    if nu_glob::is_glob_with_backend(path) || path.contains('`') {
        // expand home `~` for https://github.com/nushell/nushell/issues/13905
        let pathbuf = nu_path::expand_tilde(path);
        let path = pathbuf.to_string_lossy();
        if path.contains('\'') {
            // decide to use double quotes
            // Path as Debug will do the escaping for `"`, `\`
            Some(format!("{path:?}"))
        } else {
            Some(format!("'{path}'"))
        }
    } else {
        let contaminated =
            path.contains(['\'', '"', ' ', '#', '(', ')', '{', '}', '[', ']', '|', ';']);
        let maybe_flag = path.starts_with('-');
        let maybe_variable = path.starts_with('$');
        let maybe_number = path.parse::<f64>().is_ok();
        if contaminated || maybe_flag || maybe_variable || maybe_number {
            Some(format!("`{path}`"))
        } else {
            None
        }
    }
}

pub struct AdjustView {
    pub prefix: String,
    pub span: Span,
    pub readjusted: bool,
}

pub fn adjust_if_intermediate(
    prefix: &str,
    working_set: &StateWorkingSet,
    mut span: nu_protocol::Span,
) -> AdjustView {
    let span_contents = String::from_utf8_lossy(working_set.get_span_contents(span)).into_owned();
    let mut prefix = prefix.to_string();

    // A difference of 1 because of the cursor's unicode code point in between.
    // Using .chars().count() because unicode and Windows.
    let readjusted = span_contents.chars().count() - prefix.chars().count() > 1;
    if readjusted {
        let remnant: String = span_contents
            .chars()
            .skip(prefix.chars().count() + 1)
            .take_while(|&c| !is_separator(c))
            .collect();
        prefix.push_str(&remnant);
        span = Span::new(span.start, span.start + prefix.chars().count() + 1);
    }
    AdjustView {
        prefix,
        span,
        readjusted,
    }
}

/// Collapse multiple ".." components into n-dots.
///
/// It performs the reverse operation of `expand_ndots`, collapsing sequences of ".." into n-dots,
/// such as "..." and "....".
///
/// The resulting path will use platform-specific path separators, regardless of what path separators were used in the input.
fn collapse_ndots(path: PathBuiltFromString) -> PathBuiltFromString {
    let mut result = PathBuiltFromString {
        parts: Vec::with_capacity(path.parts.len()),
        isdir: path.isdir,
    };

    let mut dot_count = 0;

    for part in path.parts {
        if &part.text == ".." {
            dot_count += 1;
        } else {
            if dot_count > 0 {
                result.parts.push(MatchedPart {
                    text: ".".repeat(dot_count + 1),
                    match_indices: Vec::new(),
                });
                dot_count = 0;
            }
            result.parts.push(part);
        }
    }

    // Add any remaining dots
    if dot_count > 0 {
        result.parts.push(MatchedPart {
            text: ".".repeat(dot_count + 1),
            match_indices: Vec::new(),
        });
    }

    result
}
