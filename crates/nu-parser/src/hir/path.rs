use crate::hir::Expression;
use derive_new::new;
use getset::{Getters, MutGetters};
use nu_protocol::PathMember;
use nu_source::{b, DebugDocBuilder, PrettyDebug, PrettyDebugWithSource};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Getters,
    MutGetters,
    Serialize,
    Deserialize,
    new,
)]
#[get = "pub"]
pub struct Path {
    head: Expression,
    #[get_mut = "pub(crate)"]
    tail: Vec<PathMember>,
}

impl PrettyDebugWithSource for Path {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        self.head.pretty_debug(source)
            + b::operator(".")
            + b::intersperse(self.tail.iter().map(|m| m.pretty()), b::operator("."))
    }
}

impl Path {
    pub(crate) fn parts(self) -> (Expression, Vec<PathMember>) {
        (self.head, self.tail)
    }
}

// WIP: find a better place for these functions.

use libc::getpwnam;
use std::error::Error;
use std::ffi::{CStr, CString, NulError};
use std::fmt;
use std::path::PathBuf;

type HomedirResult = Result<Option<String>, UsernameError<NulError>>;

/// Matches exising user names
///
/// Used to resolve `~username` syntax, may fail if the username contains a null with
/// `NulError`. Otherwise will return a `String` with the user dir (not checked to be a
/// valid path) or `None` if there is no matching username.
pub(crate) fn username_homedir(name: &str) -> HomedirResult {
    let cname = CString::new(name).map_err(|e| UsernameError {
        username: name.into(),
        cause: e,
    })?;
    fn get_dir_from_username(cname: CString) -> Option<String> {
        let pwd = unsafe { getpwnam(cname.as_ptr()) };
        if pwd.is_null() {
            None
        } else {
            let dir = unsafe {
                let pwd = *pwd; // already checked to be not null
                CStr::from_ptr(pwd.pw_dir).to_string_lossy().into_owned()
            };
            Some(dir)
        }
    }
    Ok(get_dir_from_username(cname))
}

/// Represents a username lookup error.
///
/// This error is returned by `username_homedir()` function. The original error is
/// provided in the `cause` field, while `username` contains the name of a user whose
/// lookup caused the error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsernameError<E> {
    /// The name of the problematic username.
    pub username: String,
    /// The original error returned by the lookup function.
    pub cause: E,
}

impl<E: fmt::Display> fmt::Display for UsernameError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "error looking user '{}' up: {}",
            self.username, self.cause
        )
    }
}

impl<E: Error> Error for UsernameError<E> {
    fn description(&self) -> &str {
        "Homedir lookup error"
    }
    fn cause(&self) -> Option<&dyn Error> {
        Some(&self.cause)
    }
}

pub(crate) fn expand_tilde_with_context(
    path: &str,
    homedir: &dyn Fn() -> Option<PathBuf>,
    username_homedir: &dyn Fn(&str) -> Option<String>,
) -> PathBuf {
    if !path.starts_with('~') {
        path.into()
    } else {
        let home = homedir();
        match (path, home) {
            ("~", Some(home)) => home,
            (path, Some(home)) if path.starts_with("~/") => {
                path.replacen("~", &home.to_string_lossy(), 1).into()
            }
            (path, _) => {
                let end = path.find('/').unwrap_or_else(|| path.len());
                let name = &path[1..end];
                if let Some(home) = &username_homedir(name) {
                    path.replacen(&path[0..end], &home, 1).into()
                } else {
                    path.into()
                }
            }
        }
    }
}

pub fn expand_tilde(path: &str, homedir: &dyn Fn() -> Option<PathBuf>) -> PathBuf {
    expand_tilde_with_context(path, homedir, &|name| {
        dbg!(username_homedir(name).unwrap_or(None))
    })
}

// </WIP>
