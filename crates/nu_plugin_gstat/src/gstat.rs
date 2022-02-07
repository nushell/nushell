use git2::{Branch, BranchType, DescribeOptions, Repository};
use nu_plugin::LabeledError;
use nu_protocol::{Span, Spanned, Value};
use std::fmt::Write;
use std::ops::BitAnd;
use std::path::PathBuf;

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

    pub fn usage() -> &'static str {
        "Usage: gstat"
    }

    pub fn gstat(
        &self,
        value: &Value,
        path: Option<Spanned<String>>,
        span: &Span,
    ) -> Result<Value, LabeledError> {
        // use std::any::Any;
        // eprintln!("input type: {:?} value: {:#?}", &value.type_id(), &value);
        // eprintln!("path type: {:?} value: {:#?}", &path.type_id(), &path);

        // This is a flag to let us know if we're using the input value (value)
        // or using the path specified (path)
        let mut using_input_value = false;

        // let's get the input value as a string
        let piped_value = match value.as_string() {
            Ok(s) => {
                using_input_value = true;
                s
            }
            _ => String::new(),
        };

        // now let's get the path string
        let mut a_path = match path {
            Some(p) => {
                // should we check for input and path? nah.
                using_input_value = false;
                p
            }
            None => Spanned {
                item: ".".to_string(),
                span: *span,
            },
        };

        // If there was no path specified and there is a piped in value, let's use the piped in value
        if a_path.item == "." && piped_value.chars().count() > 0 {
            a_path.item = piped_value;
        }

        // This path has to exist
        // TODO: If the path is relative, it will be expanded using `std::env::current_dir` and not
        // the "PWD" environment variable. We would need a way to read the engine's environment
        // variables here.
        if !std::path::Path::new(&a_path.item).exists() {
            return Err(LabeledError {
                label: "error with path".to_string(),
                msg: format!("path does not exist [{}]", &a_path.item),
                span: if using_input_value {
                    Some(value.span().expect("unable to get value span"))
                } else {
                    Some(a_path.span)
                },
            });
        }

        let metadata = match std::fs::metadata(&a_path.item) {
            Ok(md) => md,
            Err(e) => {
                return Err(LabeledError {
                    label: "error with metadata".to_string(),
                    msg: format!(
                        "unable to get metadata for [{}], error: {}",
                        &a_path.item, e
                    ),
                    span: if using_input_value {
                        Some(value.span().expect("unable to get value span"))
                    } else {
                        Some(a_path.span)
                    },
                });
            }
        };

        // This path has to be a directory
        if !metadata.is_dir() {
            return Err(LabeledError {
                label: "error with directory".to_string(),
                msg: format!("path is not a directory [{}]", &a_path.item),
                span: if using_input_value {
                    Some(value.span().expect("unable to get value span"))
                } else {
                    Some(a_path.span)
                },
            });
        }

        let repo_path = match PathBuf::from(&a_path.item).canonicalize() {
            Ok(p) => p,
            Err(e) => {
                return Err(LabeledError {
                    label: format!("error canonicalizing [{}]", a_path.item),
                    msg: e.to_string(),
                    span: if using_input_value {
                        Some(value.span().expect("unable to get value span"))
                    } else {
                        Some(a_path.span)
                    },
                });
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

        let mut desc_opts = DescribeOptions::new();
        desc_opts.describe_tags();

        let tag = if let Ok(Ok(s)) = repo.describe(&desc_opts).map(|d| d.format(None)) {
            s
        } else {
            "no_tag".to_string()
        };

        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("idx_added_staged".into());
        vals.push(Value::Int {
            val: stats.idx_added_staged as i64,
            span: *span,
        });
        cols.push("idx_modified_staged".into());
        vals.push(Value::Int {
            val: stats.idx_modified_staged as i64,
            span: *span,
        });
        cols.push("idx_deleted_staged".into());
        vals.push(Value::Int {
            val: stats.idx_deleted_staged as i64,
            span: *span,
        });
        cols.push("idx_renamed".into());
        vals.push(Value::Int {
            val: stats.idx_renamed as i64,
            span: *span,
        });
        cols.push("idx_type_changed".into());
        vals.push(Value::Int {
            val: stats.idx_type_changed as i64,
            span: *span,
        });
        cols.push("wt_untracked".into());
        vals.push(Value::Int {
            val: stats.wt_untracked as i64,
            span: *span,
        });
        cols.push("wt_modified".into());
        vals.push(Value::Int {
            val: stats.wt_modified as i64,
            span: *span,
        });
        cols.push("wt_deleted".into());
        vals.push(Value::Int {
            val: stats.wt_deleted as i64,
            span: *span,
        });
        cols.push("wt_type_changed".into());
        vals.push(Value::Int {
            val: stats.wt_type_changed as i64,
            span: *span,
        });
        cols.push("wt_renamed".into());
        vals.push(Value::Int {
            val: stats.wt_renamed as i64,
            span: *span,
        });
        cols.push("ignored".into());
        vals.push(Value::Int {
            val: stats.ignored as i64,
            span: *span,
        });
        cols.push("conflicts".into());
        vals.push(Value::Int {
            val: stats.conflicts as i64,
            span: *span,
        });
        cols.push("ahead".into());
        vals.push(Value::Int {
            val: stats.ahead as i64,
            span: *span,
        });
        cols.push("behind".into());
        vals.push(Value::Int {
            val: stats.behind as i64,
            span: *span,
        });
        cols.push("stashes".into());
        vals.push(Value::Int {
            val: stats.stashes as i64,
            span: *span,
        });
        cols.push("repo_name".into());
        vals.push(Value::String {
            val: repo_name,
            span: *span,
        });
        cols.push("tag".into());
        vals.push(Value::String {
            val: tag,
            span: *span,
        });
        cols.push("branch".into());
        vals.push(Value::String {
            val: stats.branch,
            span: *span,
        });
        cols.push("remote".into());
        vals.push(Value::String {
            val: stats.remote,
            span: *span,
        });

        // Leave this in case we want to turn it into a table instead of a list
        // Ok(Value::List {
        //     vals: vec![Value::Record {
        //         cols,
        //         vals,
        //         span: *span,
        //     }],
        //     span: *span,
        // })

        Ok(Value::Record {
            cols,
            vals,
            span: *span,
        })
    }

    fn create_empty_git_status(&self, span: &Span) -> Value {
        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("idx_added_staged".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("idx_modified_staged".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("idx_deleted_staged".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("idx_renamed".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("idx_type_changed".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("wt_untracked".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("wt_modified".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("wt_deleted".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("wt_type_changed".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("wt_renamed".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("ignored".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("conflicts".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("ahead".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("behind".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("stashes".into());
        vals.push(Value::Int {
            val: -1,
            span: *span,
        });
        cols.push("repo_name".into());
        vals.push(Value::String {
            val: "no_repository".to_string(),
            span: *span,
        });
        cols.push("tag".into());
        vals.push(Value::String {
            val: "no_tag".to_string(),
            span: *span,
        });
        cols.push("branch".into());
        vals.push(Value::String {
            val: "no_branch".to_string(),
            span: *span,
        });
        cols.push("remote".into());
        vals.push(Value::String {
            val: "no_remote".to_string(),
            span: *span,
        });

        Value::Record {
            cols,
            vals,
            span: *span,
        }
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
}

impl Stats {
    /// Populate stats with the status of the given repository
    pub fn new(repo: &mut Repository) -> Stats {
        let mut st: Stats = Default::default();

        st.read_branch(repo);

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
                                write!(&mut id, "{:x}", byte).unwrap();
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
        self.remote = match repo.find_branch(branch, BranchType::Local) {
            Ok(branch) => {
                // Grab the upstream from the branch
                match branch.upstream() {
                    // Grab the name of the upstream if it's valid UTF-8
                    Ok(upstream) => {
                        // While we have the upstream branch, traverse the graph and count
                        // ahead-behind commits.
                        self.read_ahead_behind(repo, &branch, &upstream);

                        match upstream.name() {
                            Ok(Some(name)) => name.to_string(),
                            _ => String::new(),
                        }
                    }
                    _ => String::new(),
                }
            }
            _ => String::new(),
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
