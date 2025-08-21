use git2::{Branch, BranchType, DescribeOptions, Repository};
use nu_protocol::{IntoSpanned, LabeledError, Span, Spanned, Value, record};
use std::{fmt::Write, ops::BitAnd, path::Path};

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

        let repo_name = repo
            .path()
            .parent()
            .and_then(|p| p.file_name())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "".to_string());

        let tag = calculate_tag
            .then(|| {
                let mut desc_opts = DescribeOptions::new();
                desc_opts.describe_tags();
                repo.describe(&desc_opts).ok()?.format(None).ok()
            })
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
                "idx_added_staged" => Value::int(stats.idx_added_staged as i64, span),
                "idx_modified_staged" => Value::int(stats.idx_modified_staged as i64, span),
                "idx_deleted_staged" => Value::int(stats.idx_deleted_staged as i64, span),
                "idx_renamed" => Value::int(stats.idx_renamed as i64, span),
                "idx_type_changed" => Value::int(stats.idx_type_changed as i64, span),
                "wt_untracked" => Value::int(stats.wt_untracked as i64, span),
                "wt_modified" => Value::int(stats.wt_modified as i64, span),
                "wt_deleted" => Value::int(stats.wt_deleted as i64, span),
                "wt_type_changed" => Value::int(stats.wt_type_changed as i64, span),
                "wt_renamed" => Value::int(stats.wt_renamed as i64, span),
                "ignored" => Value::int(stats.ignored as i64, span),
                "conflicts" => Value::int(stats.conflicts as i64, span),
                "ahead" => Value::int(stats.ahead as i64, span),
                "behind" => Value::int(stats.behind as i64, span),
                "stashes" => Value::int(stats.stashes as i64, span),
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
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Stats {
    /// Number of files to be added
    pub idx_added_staged: u16,
    /// Number of staged changes to files
    pub idx_modified_staged: u16,
    /// Number of staged deletions
    pub idx_deleted_staged: u16,
    /// Number of renamed files
    pub idx_renamed: u16,
    /// Index file type change
    pub idx_type_changed: u16,

    /// Number of untracked files which are new to the repository
    pub wt_untracked: u16,
    /// Number of modified files which have not yet been staged
    pub wt_modified: u16,
    /// Number of deleted files
    pub wt_deleted: u16,
    /// Working tree file type change
    pub wt_type_changed: u16,
    /// Working tree renamed
    pub wt_renamed: u16,

    // Ignored files
    pub ignored: u16,
    /// Number of unresolved conflicts in the repository
    pub conflicts: u16,

    /// Number of commits ahead of the upstream branch
    pub ahead: u16,
    /// Number of commits behind the upstream branch
    pub behind: u16,
    /// Number of stashes on the current branch
    pub stashes: u16,
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
            .renames_head_to_index(true);

        if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
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
    fn read_branch(&mut self, repo: &Repository) {
        self.branch = match repo.head() {
            Ok(head) => {
                if let Some(name) = head.shorthand() {
                    // try to use first 8 characters or so of the ID in detached HEAD
                    if name == "HEAD" {
                        if let Ok(commit) = head.peel_to_commit() {
                            let mut id = String::new();
                            for byte in &commit.id().as_bytes()[..4] {
                                write!(&mut id, "{byte:02x}").unwrap();
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
            Err(ref err) if err.code() == git2::ErrorCode::BareRepo => "master".to_string(),
            Err(_) if repo.is_empty().unwrap_or(false) => "master".to_string(),
            Err(_) => "HEAD".to_string(),
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
        if let (Some(local), Some(upstream)) = (local.get().target(), upstream.get().target()) {
            if let Ok((ahead, behind)) = repo.graph_ahead_behind(local, upstream) {
                self.ahead = ahead as u16;
                self.behind = behind as u16;
            }
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

/// Check the bits of a flag against the value to see if they are set
#[inline]
fn check<B>(val: B, flag: B) -> bool
where
    B: BitAnd<Output = B> + PartialEq + Copy,
{
    val & flag == flag
}
