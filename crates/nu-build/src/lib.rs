use lazy_static::lazy_static;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

lazy_static! {
    static ref WORKSPACES: Mutex<BTreeMap<String, &'static Path>> = Mutex::new(BTreeMap::new());
}

// got from https://github.com/mitsuhiko/insta/blob/b113499249584cb650150d2d01ed96ee66db6b30/src/runtime.rs#L67-L88

fn get_cargo_workspace(manifest_dir: &str) -> Result<Option<&Path>, Box<dyn std::error::Error>> {
    let mut workspaces = WORKSPACES.lock()?;
    if let Some(rv) = workspaces.get(manifest_dir) {
        Ok(Some(rv))
    } else {
        #[derive(Deserialize)]
        struct Manifest {
            workspace_root: String,
        }
        let output = std::process::Command::new(env!("CARGO"))
            .arg("metadata")
            .arg("--format-version=1")
            .current_dir(manifest_dir)
            .output()?;
        let manifest: Manifest = serde_json::from_slice(&output.stdout)?;
        let path = Box::leak(Box::new(PathBuf::from(manifest.workspace_root)));
        workspaces.insert(manifest_dir.to_string(), path.as_path());
        Ok(workspaces.get(manifest_dir).cloned())
    }
}

#[derive(Deserialize)]
struct Feature {
    #[allow(unused)]
    description: String,
    enabled: bool,
}

pub fn build() -> Result<(), Box<dyn std::error::Error>> {
    let input = env::var("CARGO_MANIFEST_DIR")?;

    let all_on = env::var("NUSHELL_ENABLE_ALL_FLAGS").is_ok();
    let flags: HashSet<String> = env::var("NUSHELL_ENABLE_FLAGS")
        .map(|s| s.split(',').map(|s| s.to_string()).collect())
        .unwrap_or_else(|_| HashSet::new());

    if all_on && !flags.is_empty() {
        println!(
            "cargo:warning=Both NUSHELL_ENABLE_ALL_FLAGS and NUSHELL_ENABLE_FLAGS were set. You don't need both."
        );
    }

    let workspace = match get_cargo_workspace(&input)? {
        // If the crate is being downloaded from crates.io, it won't have a workspace root, and that's ok
        None => return Ok(()),
        Some(workspace) => workspace,
    };

    let path = Path::new(&workspace).join("features.toml");

    // If the crate is being downloaded from crates.io, it won't have a features.toml, and that's ok
    if !path.exists() {
        return Ok(());
    }

    let toml: HashMap<String, Feature> = toml::from_str(&std::fs::read_to_string(path)?)?;

    for (key, value) in toml.iter() {
        if value.enabled || all_on || flags.contains(key) {
            println!("cargo:rustc-cfg={}", key);
        }
    }

    Ok(())
}
