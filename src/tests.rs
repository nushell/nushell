mod test_conditionals;
mod test_config_path;
mod test_converters;
mod test_custom_commands;
mod test_engine;
mod test_env;
mod test_hiding;
mod test_iteration;
mod test_known_external;
mod test_math;
mod test_modules;
mod test_parser;
mod test_ranges;
mod test_regex;
mod test_strings;
mod test_table_operations;
mod test_type_check;

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
    cmd.arg(name).envs(env);

    writeln!(file, "{}", input)?;

    run_cmd_and_assert(cmd, expected)
}

#[cfg(test)]
pub fn run_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("nu")?;
    cmd.arg(name);
    cmd.env(
        "PWD",
        std::env::current_dir().expect("Can't get current dir"),
    );

    writeln!(file, "{}", input)?;

    run_cmd_and_assert(cmd, expected)
}

#[cfg(test)]
fn run_cmd_and_assert(mut cmd: Command, expected: &str) -> TestResult {
    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(output.status.success());

    assert_eq!(stdout.trim(), expected);

    Ok(())
}

#[cfg(test)]
pub fn run_test_contains(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("nu")?;
    cmd.arg(name);

    writeln!(file, "{}", input)?;

    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(output.status.success());

    assert!(stdout.contains(expected));

    Ok(())
}

#[cfg(test)]
pub fn fail_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("nu")?;
    cmd.arg(name);
    cmd.env(
        "PWD",
        std::env::current_dir().expect("Can't get current dir"),
    );

    writeln!(file, "{}", input)?;

    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(!stderr.is_empty() && stderr.contains(expected));

    Ok(())
}
