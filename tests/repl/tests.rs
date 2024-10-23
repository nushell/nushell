use assert_cmd::prelude::*;
use pretty_assertions::assert_eq;
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn run_test_with_env(input: &str, expected: &str, env: &HashMap<&str, &str>) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("nu")?;
    cmd.arg("--no-config-file");
    cmd.arg(name).envs(env);

    writeln!(file, "{input}")?;

    run_cmd_and_assert(cmd, expected)
}

#[cfg(test)]
pub fn run_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("nu")?;
    cmd.arg("--no-std-lib");
    cmd.arg("--no-config-file");
    cmd.arg(name);
    cmd.env(
        "PWD",
        std::env::current_dir().expect("Can't get current dir"),
    );

    writeln!(file, "{input}")?;

    run_cmd_and_assert(cmd, expected)
}

#[cfg(test)]
pub fn run_test_std(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("nu")?;
    cmd.arg("--no-config-file");
    cmd.arg(name);
    cmd.env(
        "PWD",
        std::env::current_dir().expect("Can't get current dir"),
    );

    writeln!(file, "{input}")?;

    run_cmd_and_assert(cmd, expected)
}

#[cfg(test)]
fn run_cmd_and_assert(mut cmd: Command, expected: &str) -> TestResult {
    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    assert!(output.status.success());

    assert_eq!(stdout.trim(), expected);

    Ok(())
}

#[cfg(test)]
pub fn run_test_contains(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("nu")?;
    cmd.arg("--no-std-lib");
    cmd.arg("--no-config-file");
    cmd.arg(name);

    writeln!(file, "{input}")?;

    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    println!("Expected output to contain: {expected}");
    assert!(output.status.success());

    assert!(stdout.contains(expected));

    Ok(())
}

#[cfg(test)]
pub fn test_ide_contains(input: &str, ide_commands: &[&str], expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("nu")?;
    cmd.arg("--no-std-lib");
    cmd.arg("--no-config-file");
    for ide_command in ide_commands {
        cmd.arg(ide_command);
    }
    cmd.arg(name);

    writeln!(file, "{input}")?;

    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");
    println!("Expected output to contain: {expected}");

    assert!(output.status.success());

    assert!(stdout.contains(expected));

    Ok(())
}

#[cfg(test)]
pub fn fail_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("nu")?;
    cmd.arg("--no-std-lib");
    cmd.arg("--no-config-file");
    cmd.arg(name);
    cmd.env(
        "PWD",
        std::env::current_dir().expect("Can't get current dir"),
    );

    writeln!(file, "{input}")?;

    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");
    println!("Expected error to contain: {expected}");

    assert!(!stderr.is_empty() && stderr.contains(expected));

    Ok(())
}
