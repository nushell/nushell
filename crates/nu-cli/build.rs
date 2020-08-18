use std::path::Path;
use std::{env, fs, io};

fn main() -> Result<(), io::Error> {
    let out_dir = env::var_os("OUT_DIR").expect(
    "\
        OUT_DIR environment variable not found. \
        OUT_DIR is guaranteed to to exist in a build script by cargo - see \
        https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts\
    ");

    let latest_commit_hash = latest_commit_hash(env::current_dir()?).unwrap_or_default();

    let commit_hash_path = Path::new(&out_dir).join("git_commit_hash");
    fs::write(commit_hash_path, latest_commit_hash)?;

    Ok(())
}

#[allow(unused_variables)]
fn latest_commit_hash<P: AsRef<Path>>(dir: P) -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(feature = "git2")]
    {
        use git2::Repository;
        let dir = dir.as_ref();
        Ok(Repository::discover(dir)?
            .head()?
            .peel_to_commit()?
            .id()
            .to_string())
    }
    #[cfg(not(feature = "git2"))]
    {
        Ok(String::new())
    }
}
