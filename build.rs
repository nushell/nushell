use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::path::Path;

#[derive(Deserialize)]
struct Feature {
    #[allow(unused)]
    description: String,
    enabled: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = env::var("CARGO_MANIFEST_DIR").unwrap();
    let all_on = env::var("NUSHELL_ENABLE_ALL_FLAGS").is_ok();
    let flags: HashSet<String> = env::var("NUSHELL_ENABLE_FLAGS")
        .map(|s| s.split(",").map(|s| s.to_string()).collect())
        .unwrap_or_else(|_| HashSet::new());

    if all_on && !flags.is_empty() {
        println!(
            "cargo:warning={}",
            "Both NUSHELL_ENABLE_ALL_FLAGS and NUSHELL_ENABLE_FLAGS were set. You don't need both."
        );
    }

    let path = Path::new(&input).join("features.toml");

    let toml: HashMap<String, Feature> = toml::from_str(&std::fs::read_to_string(path)?)?;

    for (key, value) in toml.iter() {
        if value.enabled == true || all_on || flags.contains(key) {
            println!("cargo:rustc-cfg={}", key);
        }
    }

    Ok(())
}
