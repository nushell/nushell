use std::path::PathBuf;

use nu_cli::NuCompleter;
use nu_command::create_default_context;
use nu_protocol::engine::{EngineState, Stack};
use nu_test_support::fs;
use reedline::{Completer, Suggestion};
const SEP: char = std::path::MAIN_SEPARATOR;

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
        file(dir.clone().join("nushell")),
        folder(dir.clone().join("test_a")),
        folder(dir.clone().join("test_b")),
        folder(dir.clone().join("another")),
        file(dir.clone().join(".hidden_file")),
        folder(dir.clone().join(".hidden_folder")),
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

    // Create a default engine
    (dir.clone(), dir_str, create_default_context(dir))
}

// match a list of suggestions with the expected values
// skipping the order (for now) due to some issue with sorting behaving
// differently for each OS
fn match_suggestions(expected: Vec<String>, suggestions: Vec<Suggestion>) {
    suggestions.into_iter().for_each(|it| {
        let items = expected.clone();
        let result = items.into_iter().find(|x| x == &it.value);

        match result {
            Some(val) => assert_eq!(val, it.value),
            None => panic!("the path {} is not expected", it.value),
        }
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
