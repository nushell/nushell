pub mod support;

use nu_cli::NuCompleter;
use reedline::Completer;
use support::{file, folder, match_suggestions, new_engine};

#[test]
fn file_completions() {
    // Create a new engine
    let (dir, dir_str, engine, stack) = new_engine();

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
        file(dir.join("custom_completion.nu")),
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
fn command_ls_completion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = format!("ls ");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder/".to_string(),
    ];

    match_suggestions(expected_paths, suggestions)
}
#[test]
fn command_open_completion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = format!("open ");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder/".to_string(),
    ];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn command_rm_completion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = format!("rm ");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder/".to_string(),
    ];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn command_cp_completion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = format!("cp ");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder/".to_string(),
    ];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn command_save_completion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = format!("save ");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder/".to_string(),
    ];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn command_touch_completion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = format!("touch ");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder/".to_string(),
    ];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn command_watch_completion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = format!("watch ");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder/".to_string(),
    ];

    match_suggestions(expected_paths, suggestions)
}
