use std::path::Path;
use std::{env, fs, io};

use git2::Repository;

#[derive(Debug)]
enum Error {
    IoError(io::Error),
    GitError(git2::Error),
}

impl From<git2::Error> for Error {
    fn from(git_error: git2::Error) -> Self {
        Self::GitError(git_error)
    }
}

impl From<io::Error> for Error {
    fn from(io_error: io::Error) -> Self {
        Self::IoError(io_error)
    }
}

fn main() -> Result<(), Error> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let latest_commit_hash = Repository::discover(env::current_dir()?)?
        .head()?
        .peel_to_commit()?
        .id()
        .to_string();

    let commit_hash_path = Path::new(&out_dir).join("git_commit_hash");
    fs::write(commit_hash_path, latest_commit_hash)?;

    Ok(())
}
