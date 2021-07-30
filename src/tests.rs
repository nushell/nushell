use assert_cmd::prelude::*;
use pretty_assertions::assert_eq;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(test)]
fn run_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("engine-q")?;
    cmd.arg(name);

    writeln!(file, "{}", input)?;

    let output = cmd.output()?;

    assert!(output.status.success());

    let output = String::from_utf8_lossy(&output.stdout).to_string();

    assert_eq!(output.trim(), expected);

    Ok(())
}

#[cfg(test)]
fn fail_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("engine-q")?;
    cmd.arg(name);

    writeln!(file, "{}", input)?;

    let output = cmd.output()?;

    let output = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(output.contains("Error:"));
    assert!(output.contains(expected));

    Ok(())
}

#[test]
fn add_simple() -> TestResult {
    run_test("3 + 4", "7")
}

#[test]
fn add_simple2() -> TestResult {
    run_test("3 + 4 + 9", "16")
}

#[test]
fn broken_math() -> TestResult {
    fail_test("3 + ", "Incomplete")
}
