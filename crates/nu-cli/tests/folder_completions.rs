pub mod support;

use std::path::PathBuf;

use nu_cli::NuCompleter;
use reedline::Completer;
use support::{folder, match_suggestions, new_engine};

#[test]
fn folder_completions() {
    // Create a new engine
    let (dir, dir_str, engine, stack) = new_engine();

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

#[test]
fn folder_completions_with_dots() {
    // Create a new engine
    let (dir, dir_str, engine, stack) = new_engine();

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for the current folder
    let parent_dir = PathBuf::from(dir_str).join("..");
    let target_dir = format!("cd {}", parent_dir.to_str().unwrap());
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![
        folder(dir.join("..").join("formats")),
        folder(dir.join("..").join("playground")),
        folder(dir.join("..").join("completions")),
    ];

    // Match the results
    match_suggestions(expected_paths, suggestions);
}
