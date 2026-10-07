use std::process::Command;

fn main() {
    // The git commit a pickle is made with (see `src/pickle.rs`), the same value nu-cmd-lang's
    // build script gives `version`. It can be set from outside, such as with nix.
    let hash = get_git_hash().unwrap_or(
        option_env!("NU_COMMIT_HASH")
            .unwrap_or_default()
            .to_string(),
    );
    println!("cargo:rustc-env=NU_COMMIT_HASH={hash}");
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
