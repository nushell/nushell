use git2::{Branch, BranchType, Oid, Repository};
use nu_protocol::{IntoSpanned, LabeledError, Span, Spanned, Value, record};
use std::{collections::HashMap, fmt::Write, ops::BitAnd, path::Path};

// git status
// https://github.com/git/git/blob/9875c515535860450bafd1a177f64f0a478900fa/Documentation/git-status.txt

// git status borrowed from here and tweaked
// https://github.com/glfmn/glitter/blob/master/lib/git.rs

#[derive(Default)]
pub struct GStat;

impl GStat {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn gstat(
        &self,
        value: &Value,
        current_dir: &str,
        path: Option<Spanned<String>>,
        calculate_tag: bool,
        span: Span,
    ) -> Result<Value, LabeledError> {
        // use std::any::Any;
        // eprintln!("input type: {:?} value: {:#?}", &value.type_id(), &value);
        // eprintln!("path type: {:?} value: {:#?}", &path.type_id(), &path);

        // If the path isn't set, get it from input, and failing that, set to "."
        let path = match path {
            Some(path) => path,
            None => {
                if !value.is_nothing() {
                    value.coerce_string()?.into_spanned(value.span())
                } else {
                    String::from(".").into_spanned(span)
                }
            }
        };

        // Make the path absolute based on the current_dir
        let absolute_path = Path::new(current_dir).join(&path.item);

        // This path has to exist
        if !absolute_path.exists() {
            return Err(LabeledError::new("error with path").with_label(
                format!("path does not exist [{}]", absolute_path.display()),
                path.span,
            ));
        }
        let metadata = std::fs::metadata(&absolute_path).map_err(|e| {
            LabeledError::new("error with metadata").with_label(
                format!(
                    "unable to get metadata for [{}], error: {}",
                    absolute_path.display(),
                    e
                ),
                path.span,
            )
        })?;

        // This path has to be a directory
        if !metadata.is_dir() {
            return Err(LabeledError::new("error with directory").with_label(
                format!("path is not a directory [{}]", absolute_path.display()),
                path.span,
            ));
        }

        let repo_path = match absolute_path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                return Err(LabeledError::new(format!(
                    "error canonicalizing [{}]",
                    absolute_path.display()
                ))
                .with_label(e.to_string(), path.span));
            }
        };

        let (stats, repo) = if let Ok(mut repo) = Repository::discover(repo_path) {
            (Stats::new(&mut repo), repo)
        } else {
            return Ok(self.create_empty_git_status(span));
        };

        // Name the repo after its working directory. The git dir's parent only works for a plain
        // checkout: a linked worktree's git dir is `.git/worktrees/<name>` and a submodule's is
        // `.git/modules/<name>`. A bare repo has no working directory, so use the git dir itself.
        let repo_name = repo
            .workdir()
            .unwrap_or_else(|| repo.path())
            .file_name()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let tag = calculate_tag
            .then(|| nearest_tag(&repo))
            .flatten()
            .unwrap_or_else(|| "no_tag".to_string());

        // Leave this in case we want to turn it into a table instead of a list
        // Ok(Value::List {
        //     vals: vec![Value::Record {
        //         cols,
        //         vals,
        //         span: *span,
        //     }],
        //     span: *span,
        // })

        Ok(Value::record(
            record! {
                "idx_added_staged" => Value::int(stats.idx_added_staged, span),
                "idx_modified_staged" => Value::int(stats.idx_modified_staged, span),
                "idx_deleted_staged" => Value::int(stats.idx_deleted_staged, span),
                "idx_renamed" => Value::int(stats.idx_renamed, span),
                "idx_type_changed" => Value::int(stats.idx_type_changed, span),
                "wt_untracked" => Value::int(stats.wt_untracked, span),
                "wt_modified" => Value::int(stats.wt_modified, span),
                "wt_deleted" => Value::int(stats.wt_deleted, span),
                "wt_type_changed" => Value::int(stats.wt_type_changed, span),
                "wt_renamed" => Value::int(stats.wt_renamed, span),
                "ignored" => Value::int(stats.ignored, span),
                "conflicts" => Value::int(stats.conflicts, span),
                "ahead" => Value::int(stats.ahead, span),
                "behind" => Value::int(stats.behind, span),
                "stashes" => Value::int(stats.stashes, span),
                "repo_name" => Value::string(repo_name, span),
                "tag" => Value::string(tag, span),
                "branch" => Value::string(stats.branch, span),
                "remote" => Value::string(stats.remote, span),
                "state" => Value::string(stats.state, span),
            },
            span,
        ))
    }

    fn create_empty_git_status(&self, span: Span) -> Value {
        Value::record(
            record! {
                "idx_added_staged" => Value::int(-1, span),
                "idx_modified_staged" => Value::int(-1, span),
                "idx_deleted_staged" => Value::int(-1, span),
                "idx_renamed" => Value::int(-1, span),
                "idx_type_changed" => Value::int(-1, span),
                "wt_untracked" => Value::int(-1, span),
                "wt_modified" => Value::int(-1, span),
                "wt_deleted" => Value::int(-1, span),
                "wt_type_changed" => Value::int(-1, span),
                "wt_renamed" => Value::int(-1, span),
                "ignored" => Value::int(-1, span),
                "conflicts" => Value::int(-1, span),
                "ahead" => Value::int(-1, span),
                "behind" => Value::int(-1, span),
                "stashes" => Value::int(-1, span),
                "repo_name" => Value::string("no_repository", span),
                "tag" => Value::string("no_tag", span),
                "branch" => Value::string("no_branch", span),
                "remote" => Value::string("no_remote", span),
                "state" => Value::string("no_state", span),
            },
            span,
        )
    }
}

/// Stats which the interpreter uses to populate the gist expression
///
/// The counts from the status list (`idx_*`, `wt_*`, `ignored` and `conflicts`) are -1 when
/// libgit2 cannot compute the status, for example for a split or sparse index or a bare
/// repository, so that an unknown status never reads as a clean tree.
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Stats {
    /// Number of files to be added
    pub idx_added_staged: i64,
    /// Number of staged changes to files
    pub idx_modified_staged: i64,
    /// Number of staged deletions
    pub idx_deleted_staged: i64,
    /// Number of renamed files
    pub idx_renamed: i64,
    /// Index file type change
    pub idx_type_changed: i64,

    /// Number of untracked files which are new to the repository
    pub wt_untracked: i64,
    /// Number of modified files which have not yet been staged
    pub wt_modified: i64,
    /// Number of deleted files
    pub wt_deleted: i64,
    /// Working tree file type change
    pub wt_type_changed: i64,
    /// Number of unstaged renames: a deleted tracked file paired with a similar untracked file,
    /// which are then not counted in `wt_deleted` and `wt_untracked`. A renamed file that was
    /// also edited counts in `wt_modified` too.
    pub wt_renamed: i64,

    /// Number of ignored entries; an ignored directory counts once, without walking into it
    pub ignored: i64,
    /// Number of unresolved conflicts in the repository
    pub conflicts: i64,

    /// Number of commits ahead of the upstream branch
    pub ahead: i64,
    /// Number of commits behind the upstream branch
    pub behind: i64,
    /// Number of stashes on the current branch
    pub stashes: i64,
    /// The branch name or other stats of the HEAD pointer
    pub branch: String,
    /// The of the upstream branch
    pub remote: String,

    /// State of the repository (Clean, Merge, Rebase, etc.)
    /// See all states at https://docs.rs/git2/latest/git2/enum.RepositoryState.html
    pub state: String,
}

impl Stats {
    /// Populate stats with the status of the given repository
    pub fn new(repo: &mut Repository) -> Stats {
        let mut st: Stats = Default::default();

        st.read_branch(repo);
        st.state = format!("{:?}", repo.state()).to_lowercase();

        let mut opts = git2::StatusOptions::new();

        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(true)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true);

        // Rename detection reads untracked files, and one unreadable file fails the whole list.
        // Retry without it so the other counts survive.
        if let Ok(statuses) = repo.statuses(Some(&mut opts)).or_else(|_| {
            opts.renames_index_to_workdir(false);
            repo.statuses(Some(&mut opts))
        }) {
            for status in statuses.iter() {
                let flags = status.status();

                if check(flags, git2::Status::INDEX_NEW) {
                    st.idx_added_staged += 1;
                }
                if check(flags, git2::Status::INDEX_MODIFIED) {
                    st.idx_modified_staged += 1;
                }
                if check(flags, git2::Status::INDEX_DELETED) {
                    st.idx_deleted_staged += 1;
                }
                if check(flags, git2::Status::INDEX_RENAMED) {
                    st.idx_renamed += 1;
                }
                if check(flags, git2::Status::INDEX_TYPECHANGE) {
                    st.idx_type_changed += 1;
                }

                if check(flags, git2::Status::WT_NEW) {
                    st.wt_untracked += 1;
                }
                if check(flags, git2::Status::WT_MODIFIED) {
                    st.wt_modified += 1;
                }
                if check(flags, git2::Status::WT_DELETED) {
                    st.wt_deleted += 1;
                }
                if check(flags, git2::Status::WT_TYPECHANGE) {
                    st.wt_type_changed += 1;
                }
                if check(flags, git2::Status::WT_RENAMED) {
                    st.wt_renamed += 1;
                }

                if check(flags, git2::Status::IGNORED) {
                    st.ignored += 1;
                }
                if check(flags, git2::Status::CONFLICTED) {
                    st.conflicts += 1;
                }
            }
        } else {
            // libgit2 cannot read every index (split or sparse indexes, a damaged file), and a
            // bare repository has no working tree. Report the counts as unknown instead of as a
            // clean tree.
            for count in [
                &mut st.idx_added_staged,
                &mut st.idx_modified_staged,
                &mut st.idx_deleted_staged,
                &mut st.idx_renamed,
                &mut st.idx_type_changed,
                &mut st.wt_untracked,
                &mut st.wt_modified,
                &mut st.wt_deleted,
                &mut st.wt_type_changed,
                &mut st.wt_renamed,
                &mut st.ignored,
                &mut st.conflicts,
            ] {
                *count = -1;
            }
        }

        let _ = repo.stash_foreach(|_, &_, &_| {
            st.stashes += 1;
            true
        });

        st
    }

    /// Read the branch-name of the repository
    ///
    /// If in detached head, grab the first few characters of the commit ID if possible, otherwise
    /// simply provide HEAD as the branch name.  This is to mimic the behaviour of `git status`.
    /// A branch with no commits yet is named after the branch `HEAD` points to.
    fn read_branch(&mut self, repo: &Repository) {
        self.branch = match repo.head() {
            Ok(head) => {
                if let Ok(name) = head.shorthand() {
                    // try to use first 8 characters or so of the ID in detached HEAD
                    if name == "HEAD" {
                        if let Ok(commit) = head.peel_to_commit() {
                            let mut id = String::new();
                            for byte in &commit.id().as_bytes()[..4] {
                                write!(&mut id, "{byte:02x}").ok();
                            }
                            id
                        } else {
                            "HEAD".to_string()
                        }
                        // Grab the branch from the reference
                    } else {
                        let branch = name.to_string();
                        // Since we have a branch name, look for the name of the upstream branch
                        self.read_upstream_name(repo, &branch);
                        branch
                    }
                } else {
                    "HEAD".to_string()
                }
            }
            // `HEAD` names a branch that doesn't exist yet, as in a new repository. Read the name
            // from `HEAD` itself rather than assuming `master`, since `init.defaultBranch` or
            // `git init --initial-branch` can pick any name.
            Err(_) => repo
                .find_reference("HEAD")
                .ok()
                .and_then(|head| {
                    head.symbolic_target()
                        .ok()
                        .flatten()
                        .map(|target| target.trim_start_matches("refs/heads/").to_string())
                })
                .unwrap_or_else(|| "HEAD".to_string()),
        };
    }

    /// Read name of the upstream branch
    fn read_upstream_name(&mut self, repo: &Repository, branch: &str) {
        // First grab branch from the name
        self.remote = if let Ok(branch) = repo.find_branch(branch, BranchType::Local) {
            // Grab the upstream from the branch
            if let Ok(upstream) = branch.upstream() {
                // While we have the upstream branch, traverse the graph and count
                // ahead-behind commits.
                self.read_ahead_behind(repo, &branch, &upstream);

                if let Ok(Some(name)) = upstream.name() {
                    name.to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
    }

    /// Read ahead-behind information between the local and upstream branches
    fn read_ahead_behind(&mut self, repo: &Repository, local: &Branch, upstream: &Branch) {
        if let (Some(local), Some(upstream)) = (local.get().target(), upstream.get().target())
            && let Ok((ahead, behind)) = repo.graph_ahead_behind(local, upstream)
        {
            self.ahead = ahead as i64;
            self.behind = behind as i64;
        }
    }
}

// impl AddAssign for Stats {
//     fn add_assign(&mut self, rhs: Self) {
//         self.untracked += rhs.untracked;
//         self.added_staged += rhs.added_staged;
//         self.modified += rhs.modified;
//         self.modified_staged += rhs.modified_staged;
//         self.renamed += rhs.renamed;
//         self.deleted += rhs.deleted;
//         self.deleted_staged += rhs.deleted_staged;
//         self.ahead += rhs.ahead;
//         self.behind += rhs.behind;
//         self.conflicts += rhs.conflicts;
//         self.stashes += rhs.stashes;
//     }
// }

/// The closest tag reachable from `HEAD`, formatted like `git describe --tags`: the tag itself
/// when `HEAD` is tagged, otherwise `<tag>-<commits since the tag>-g<abbreviated hash>`.
///
/// libgit2's own `describe` walks the entire commit history looking for candidate tags, which
/// cost hundreds of milliseconds on large repositories every time a prompt was drawn. Tags are
/// few, so this maps every tag to the commit it points at once and then walks back from `HEAD`
/// only until the first tagged commit.
fn nearest_tag(repo: &Repository) -> Option<String> {
    let mut tagged_commits: HashMap<Oid, String> = HashMap::new();
    for reference in repo.references_glob("refs/tags/*").ok()?.flatten() {
        if let (Ok(name), Ok(commit)) = (reference.shorthand(), reference.peel_to_commit()) {
            tagged_commits
                .entry(commit.id())
                .or_insert_with(|| name.to_string());
        }
    }
    if tagged_commits.is_empty() {
        return None;
    }

    let head = repo.head().ok()?.peel_to_commit().ok()?.id();
    if let Some(tag) = tagged_commits.get(&head) {
        return Some(tag.clone());
    }

    // The default walk order yields commits lazily; a topological sort would load the whole
    // history first.
    let mut walk = repo.revwalk().ok()?;
    walk.push(head).ok()?;
    let tagged = walk.flatten().find(|id| tagged_commits.contains_key(id))?;

    // Like `git describe`, count the commits reachable from HEAD but not from the tag.
    let mut walk = repo.revwalk().ok()?;
    walk.push(head).ok()?;
    walk.hide(tagged).ok()?;
    let distance = walk.count();

    let abbreviated_head = head.to_string().chars().take(7).collect::<String>();
    Some(format!(
        "{}-{distance}-g{abbreviated_head}",
        tagged_commits[&tagged]
    ))
}

/// Check the bits of a flag against the value to see if they are set
#[inline]
fn check<B>(val: B, flag: B) -> bool
where
    B: BitAnd<Output = B> + PartialEq + Copy,
{
    val & flag == flag
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{IndexAddOption, RepositoryInitOptions, Signature};
    use nu_protocol::Record;
    use std::fs;

    /// Creates a repository in `dir` whose `HEAD` is `branch`, and commits `files` if there are any.
    fn init_repo(dir: &Path, branch: &str, files: &[(&str, &str)]) -> Repository {
        let repo =
            Repository::init_opts(dir, RepositoryInitOptions::new().initial_head(branch)).unwrap();
        if !files.is_empty() {
            for (name, contents) in files {
                fs::write(dir.join(name), contents).unwrap();
            }
            let mut index = repo.index().unwrap();
            index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
            index.write().unwrap();
            let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
            let signature = Signature::now("gstat", "gstat@example.com").unwrap();
            repo.commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
                .unwrap();
        }
        repo
    }

    fn gstat(dir: &Path) -> Record {
        let span = Span::test_data();
        GStat::new()
            .gstat(
                &Value::nothing(span),
                dir.to_str().unwrap(),
                None,
                false,
                span,
            )
            .unwrap()
            .into_record()
            .unwrap()
    }

    fn int(record: &Record, column: &str) -> i64 {
        record.get(column).unwrap().as_int().unwrap()
    }

    fn string<'a>(record: &'a Record, column: &str) -> &'a str {
        record.get(column).unwrap().as_str().unwrap()
    }

    #[test]
    fn counts_ignored_entries_and_unstaged_renames() {
        let dir = tempfile::tempdir().unwrap();
        let contents = "enough text for rename detection to compare\n".repeat(10);
        init_repo(
            dir.path(),
            "main",
            &[(".gitignore", "*.log\n"), ("before.txt", &contents)],
        );
        fs::rename(dir.path().join("before.txt"), dir.path().join("after.txt")).unwrap();
        fs::write(dir.path().join("build.log"), "").unwrap();

        let record = gstat(dir.path());
        assert_eq!(int(&record, "wt_renamed"), 1);
        assert_eq!(int(&record, "wt_deleted"), 0);
        assert_eq!(int(&record, "wt_untracked"), 0);
        assert_eq!(int(&record, "ignored"), 1);
    }

    #[test]
    fn unreadable_index_reports_unknown_counts() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path(), "main", &[("a.txt", "a\n")]);
        // An index header that promises one entry and then ends.
        fs::write(dir.path().join(".git/index"), b"DIRC\0\0\0\x02\0\0\0\x01").unwrap();

        let record = gstat(dir.path());
        assert_eq!(int(&record, "idx_added_staged"), -1);
        assert_eq!(int(&record, "wt_modified"), -1);
        assert_eq!(int(&record, "conflicts"), -1);
        assert_eq!(string(&record, "branch"), "main");
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_untracked_file_keeps_the_counts() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let contents = "enough text for rename detection to compare\n".repeat(10);
        init_repo(dir.path(), "main", &[("before.txt", &contents)]);
        fs::remove_file(dir.path().join("before.txt")).unwrap();
        let unreadable = dir.path().join("after.txt");
        fs::write(&unreadable, &contents).unwrap();
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();
        fs::write(dir.path().join("other.txt"), "other\n").unwrap();

        let record = gstat(dir.path());
        // Rename detection can't read `after.txt` (unless the tests run as root, which can), so
        // the move shows up either as a rename or as a deletion plus an untracked file.
        assert_eq!(int(&record, "wt_deleted") + int(&record, "wt_renamed"), 1);
        assert_eq!(int(&record, "wt_untracked") + int(&record, "wt_renamed"), 2);
    }

    #[test]
    fn unborn_branch_is_named_by_head() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path(), "trunk", &[]);
        assert_eq!(string(&gstat(dir.path()), "branch"), "trunk");
    }

    #[test]
    fn linked_worktree_is_named_after_its_directory() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("main_checkout");
        let linked = dir.path().join("linked_checkout");
        let repo = init_repo(&main, "main", &[("a.txt", "a\n")]);
        repo.worktree("linked", &linked, None).unwrap();

        assert_eq!(string(&gstat(&main), "repo_name"), "main_checkout");
        assert_eq!(string(&gstat(&linked), "repo_name"), "linked_checkout");
    }
}
