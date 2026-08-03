use std::{env, path::PathBuf, process::Command, sync::LazyLock};

fn main() {
    extract_build_profile_and_target();
}

static OUT_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var("OUT_DIR").expect("set by cargo")));

static TARGET_LIST: LazyLock<Vec<String>> = LazyLock::new(|| {
    let result = Command::new("rustc")
        .args(["--print", "target-list"])
        .output()
        .expect("calling rustc should work");
    assert!(result.status.success());
    String::from_utf8(result.stdout)
        .expect("valid utf8")
        .trim()
        .lines()
        .map(ToString::to_string)
        .collect()
});

fn extract_build_profile_and_target() {
    let mut components = OUT_DIR.components().rev();
    let _out = components.next().expect("has `out` dir");
    let _crate = components.next().expect("has dir for crate");
    let _build = components.next().expect("has `build` dir");
    let profile = components.next().expect("has profile dir");
    let profile = profile.as_os_str().to_string_lossy();
    println!("cargo::rustc-env=BUILD_PROFILE={profile}");

    if let Some(maybe_target) = components.next() {
        let maybe_target = maybe_target.as_os_str().to_string_lossy();
        if TARGET_LIST.iter().any(|target| target.as_str() == maybe_target.as_ref()) {
            println!("cargo::rustc-env=BUILD_TARGET={maybe_target}");
        }
    }
}
