use std::process::Command;

fn main() -> shadow_rs::SdResult<()> {
    // Look up the current Git commit ourselves instead of relying on shadow_rs,
    // because shadow_rs does it in a really slow-to-compile way (it builds libgit2)
    let hash = get_git_hash().expect("failed to get latest git commit hash");
    println!("cargo:rustc-env=NU_COMMIT_HASH={}", hash);

    shadow_rs::new()
}

fn get_git_hash() -> Result<String, std::io::Error> {
    let out = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    Ok(String::from_utf8(out.stdout)
        .expect("could not convert stdout to string")
        .trim()
        .to_string())
}
