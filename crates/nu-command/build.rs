use std::process::Command;

fn main() -> shadow_rs::SdResult<()> {
    // Look up the current Git commit ourselves instead of relying on shadow_rs,
    // because shadow_rs does it in a really slow-to-compile way (it builds libgit2)
    let hash = get_git_hash().unwrap_or_default();
    println!("cargo:rustc-env=NU_COMMIT_HASH={}", hash);

    shadow_rs::new()
}

fn get_git_hash() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|hash| hash.trim().to_string())
}
