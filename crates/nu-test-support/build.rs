use std::{env, path::PathBuf, sync::LazyLock};

fn main() {
    extract_build_profile();
}

static OUT_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var("OUT_DIR").expect("set by cargo")));

fn extract_build_profile() {
    let mut components = OUT_DIR.components().rev();
    let _out = components.next().expect("has `out` dir");
    let _crate = components.next().expect("has dir for crate");
    let _build = components.next().expect("has `build` dir");
    let profile = components.next().expect("has profile dir");
    let profile = profile.as_os_str().to_string_lossy();
    println!("cargo::rustc-env=BUILD_PROFILE={profile}");
}

// TODO: maybe figure out target too
