use std::{
    io::Read,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Mutex,
    },
};

static CARGO_BUILD_LOCK: Mutex<()> = Mutex::new(());
static PLUGINS_BUILT: AtomicBool = AtomicBool::new(false);

// This runs `cargo build --package nu_plugin_*` to ensure that all plugins
// have been built before plugin tests run. We use a lock to avoid multiple
// simultaneous `cargo build` invocations clobbering each other.
pub fn ensure_plugins_built() {
    let _guard = CARGO_BUILD_LOCK.lock().expect("could not get mutex lock");

    if PLUGINS_BUILT.load(Relaxed) {
        return;
    }

    let cargo_path = env!("CARGO");
    let mut arguments = vec![
        "build",
        "--workspace",
        "--bins",
        // Don't build nu, so that we only build the plugins
        "--exclude",
        "nu",
        // Exclude nu_plugin_polars, because it's not needed at this stage, and is a large build
        "--exclude",
        "nu_plugin_polars",
        "--quiet",
    ];

    let profile = std::env::var("NUSHELL_CARGO_PROFILE");
    if let Ok(profile) = &profile {
        arguments.push("--profile");
        arguments.push(profile);
    }

    let mut command = Command::new(cargo_path)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cargo build command");

    let stderr = command.stderr.take();

    let success = command
        .wait()
        .expect("failed to wait cargo build command")
        .success();

    if let Some(mut stderr) = stderr {
        let mut buffer = String::new();
        stderr
            .read_to_string(&mut buffer)
            .expect("failed to read cargo build stderr");
        if !buffer.is_empty() {
            println!("=== cargo build stderr\n{buffer}");
        }
    }

    if !success {
        panic!("cargo build failed");
    }

    PLUGINS_BUILT.store(true, Relaxed);
}
