use std::{
    io::Read,
    process::{Command, Stdio},
};

pub fn ensure_binary_present(package: &str) {
    let cargo_path = env!("CARGO");
    let mut arguments = vec!["build", "--package", package, "--quiet"];

    let profile = std::env::var("NUSHELL_CARGO_TARGET");
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
}
