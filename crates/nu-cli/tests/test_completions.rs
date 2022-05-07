use std::path::PathBuf;

use nu_cli::NuCompleter;
use nu_command::create_default_context;
use nu_protocol::{
    engine::{EngineState, Stack, StateDelta},
    Value,
};
use nu_test_support::fs;
use reedline::{Completer, Suggestion};
const SEP: char = std::path::MAIN_SEPARATOR;

#[test]
fn dotnu_completions() {
    // Create a new engine
    let (_, dir_str, engine) = new_engine();

    let mut stack = Stack::new();

    // Add pwd as env var
    stack.add_env_var(
        "PWD".to_string(),
        Value::String {
            val: dir_str.clone(),
            span: nu_protocol::Span {
                start: 0,
                end: dir_str.len(),
            },
        },
    );

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test source completion
    let completion_str = "source ".to_string();
    let suggestions = completer.complete(&completion_str, completion_str.len());

    assert_eq!(1, suggestions.len());
    assert_eq!("test_dotnu.nu", suggestions.get(0).unwrap().value);

    // Test use completion
    let completion_str = "use ".to_string();
    let suggestions = completer.complete(&completion_str, completion_str.len());

    assert_eq!(1, suggestions.len());
    assert_eq!("test_dotnu.nu", suggestions.get(0).unwrap().value);
}

#[test]
fn flag_completions() {
    // Create a new engine
    let (_, _, engine) = new_engine();

    let stack = Stack::new();

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);
    // Test completions for the 'ls' flags
    let suggestions = completer.complete("ls -".into(), 4);

    assert_eq!(12, suggestions.len());

    let expected: Vec<String> = vec![
        "--all".into(),
        "--du".into(),
        "--full-paths".into(),
        "--help".into(),
        "--long".into(),
        "--short-names".into(),
        "-a".into(),
        "-d".into(),
        "-f".into(),
        "-h".into(),
        "-l".into(),
        "-s".into(),
    ];

    // Match results
    match_suggestions(expected, suggestions);
}

#[test]
fn file_completions() {
    // Create a new engine
    let (dir, dir_str, engine) = new_engine();

    let stack = Stack::new();

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for the current folder
    let target_dir = format!("cp {}", dir_str);
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![
        file(dir.join("nushell")),
        folder(dir.join("test_a")),
        folder(dir.join("test_b")),
        folder(dir.join("another")),
        file(dir.join("test_dotnu.nu")),
        file(dir.join(".hidden_file")),
        folder(dir.join(".hidden_folder")),
    ];

    // Match the results
    match_suggestions(expected_paths, suggestions);

    // Test completions for the completions/another folder
    let target_dir = format!("cd {}", folder(dir.join("another")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![file(dir.join("another").join("newfile"))];

    // Match the results
    match_suggestions(expected_paths, suggestions);
}

#[test]
fn folder_completions() {
    // Create a new engine
    let (dir, dir_str, engine) = new_engine();

    let stack = Stack::new();

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for the current folder
    let target_dir = format!("cd {}", dir_str);
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![
        folder(dir.join("test_a")),
        folder(dir.join("test_b")),
        folder(dir.join("another")),
        folder(dir.join(".hidden_folder")),
    ];

    // Match the results
    match_suggestions(expected_paths, suggestions);
}

// creates a new engine with the current path into the completions fixtures folder
pub fn new_engine() -> (PathBuf, String, EngineState) {
    // Target folder inside assets
    let dir = fs::fixtures().join("completions");
    let mut dir_str = dir
        .clone()
        .into_os_string()
        .into_string()
        .unwrap_or_default();
    dir_str.push(SEP);

    // Create a new engine with default context
    let mut engine_state = create_default_context(&dir);

    // New stack
    let mut stack = Stack::new();

    // New delta
    let delta = StateDelta::new(&engine_state);

    // Add pwd as env var
    stack.add_env_var(
        "PWD".to_string(),
        Value::String {
            val: dir_str.clone(),
            span: nu_protocol::Span {
                start: 0,
                end: dir_str.len(),
            },
        },
    );

    // Merge delta
    let _ = engine_state.merge_delta(delta, Some(&mut stack), &dir);

    (dir.clone(), dir_str, engine_state)
}

// match a list of suggestions with the expected values
pub fn match_suggestions(expected: Vec<String>, suggestions: Vec<Suggestion>) {
    expected.iter().zip(suggestions).for_each(|it| {
        assert_eq!(it.0, &it.1.value);
    });
}

// append the separator to the converted path
pub fn folder(path: PathBuf) -> String {
    let mut converted_path = file(path);
    converted_path.push(SEP);

    converted_path
}

// convert a given path to string
pub fn file(path: PathBuf) -> String {
    path.into_os_string().into_string().unwrap_or_default()
}
