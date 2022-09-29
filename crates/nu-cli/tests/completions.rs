pub mod support;

use nu_cli::NuCompleter;
use nu_parser::parse;
use nu_protocol::engine::StateWorkingSet;
use reedline::{Completer, Suggestion};
use rstest::{fixture, rstest};
use support::{file, folder, match_suggestions, new_engine};

#[fixture]
fn completer() -> NuCompleter {
    // Create a new engine
    let (dir, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = "def tst [--mod -s] {}";
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    // Instantiate a new completer
    NuCompleter::new(std::sync::Arc::new(engine), stack)
}

#[fixture]
fn completer_strings() -> NuCompleter {
    // Create a new engine
    let (dir, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = r#"def animals [] { ["cat", "dog", "eel" ] }
    def my-command [animal: string@animals] { print $animal }"#;
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    // Instantiate a new completer
    NuCompleter::new(std::sync::Arc::new(engine), stack)
}

#[test]
fn variables_dollar_sign_with_varialblecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "$ ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    assert_eq!(7, suggestions.len());
}

#[rstest]
fn variables_double_dash_argument_with_flagcompletion(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst --", 6);
    let expected: Vec<String> = vec!["--help".into(), "--mod".into()];
    // dbg!(&expected, &suggestions);
    match_suggestions(expected, suggestions);
}

#[rstest]
fn variables_single_dash_argument_with_flagcompletion(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst -", 5);
    let expected: Vec<String> = vec!["--help".into(), "--mod".into(), "-h".into(), "-s".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn variables_command_with_commandcompletion(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-c ", 4);
    let expected: Vec<String> = vec!["my-command".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn variables_subcommands_with_customcompletion(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-command ", 11);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn variables_customcompletion_subcommands_with_customcompletion_2(
    mut completer_strings: NuCompleter,
) {
    let suggestions = completer_strings.complete("my-command ", 11);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[test]
fn dotnu_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_engine();

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test source completion
    let completion_str = "source-env ".to_string();
    let suggestions = completer.complete(&completion_str, completion_str.len());

    assert_eq!(1, suggestions.len());
    assert_eq!("custom_completion.nu", suggestions.get(0).unwrap().value);

    // Test use completion
    let completion_str = "use ".to_string();
    let suggestions = completer.complete(&completion_str, completion_str.len());

    assert_eq!(1, suggestions.len());
    assert_eq!("custom_completion.nu", suggestions.get(0).unwrap().value);
}

#[test]
#[ignore]
fn external_completer_trailing_space() {
    // https://github.com/nushell/nushell/issues/6378
    let block = "let external_completer = {|spans| $spans}";
    let input = "gh alias ".to_string();

    let suggestions = run_external_completion(block, &input);
    assert_eq!(3, suggestions.len());
    assert_eq!("gh", suggestions.get(0).unwrap().value);
    assert_eq!("alias", suggestions.get(1).unwrap().value);
    assert_eq!("", suggestions.get(2).unwrap().value);
}

#[test]
fn external_completer_no_trailing_space() {
    let block = "let external_completer = {|spans| $spans}";
    let input = "gh alias".to_string();

    let suggestions = run_external_completion(block, &input);
    assert_eq!(2, suggestions.len());
    assert_eq!("gh", suggestions.get(0).unwrap().value);
    assert_eq!("alias", suggestions.get(1).unwrap().value);
}

#[test]
fn external_completer_pass_flags() {
    let block = "let external_completer = {|spans| $spans}";
    let input = "gh api --".to_string();

    let suggestions = run_external_completion(block, &input);
    assert_eq!(3, suggestions.len());
    assert_eq!("gh", suggestions.get(0).unwrap().value);
    assert_eq!("api", suggestions.get(1).unwrap().value);
    assert_eq!("--", suggestions.get(2).unwrap().value);
}

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

    // Test completions for a file
    let target_dir = format!("cp {}", folder(dir.join("another")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![file(dir.join("another").join("newfile"))];

    // Match the results
    match_suggestions(expected_paths, suggestions);
}

#[test]
fn command_ls_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "ls ";
    let suggestions = completer.complete(target_dir, target_dir.len());

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
fn command_open_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "open ";
    let suggestions = completer.complete(target_dir, target_dir.len());

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
fn command_rm_with_globcompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "rm ";
    let suggestions = completer.complete(target_dir, target_dir.len());

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
fn command_cp_with_globcompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "cp ";
    let suggestions = completer.complete(target_dir, target_dir.len());

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
fn command_save_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "save ";
    let suggestions = completer.complete(target_dir, target_dir.len());

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
fn command_touch_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "touch ";
    let suggestions = completer.complete(target_dir, target_dir.len());

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
fn command_watch_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "watch ";
    let suggestions = completer.complete(target_dir, target_dir.len());

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
fn flag_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_engine();

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);
    // Test completions for the 'ls' flags
    let suggestions = completer.complete("ls -", 4);

    assert_eq!(14, suggestions.len());

    let expected: Vec<String> = vec![
        "--all".into(),
        "--directory".into(),
        "--du".into(),
        "--full-paths".into(),
        "--help".into(),
        "--long".into(),
        "--short-names".into(),
        "-D".into(),
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
fn folder_with_directorycompletions() {
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
fn variables_completions() {
    // Create a new engine
    let (dir, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = "let actor = { name: 'Tom Hardy', age: 44 }";
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for $nu
    let suggestions = completer.complete("$nu.", 4);

    assert_eq!(9, suggestions.len());

    let expected: Vec<String> = vec![
        "config-path".into(),
        "env-path".into(),
        "history-path".into(),
        "home-path".into(),
        "loginshell-path".into(),
        "os-info".into(),
        "pid".into(),
        "scope".into(),
        "temp-path".into(),
    ];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for $nu.h (filter)
    let suggestions = completer.complete("$nu.h", 5);

    assert_eq!(2, suggestions.len());

    let expected: Vec<String> = vec!["history-path".into(), "home-path".into()];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for custom var
    let suggestions = completer.complete("$actor.", 7);

    assert_eq!(2, suggestions.len());

    let expected: Vec<String> = vec!["age".into(), "name".into()];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for custom var (filtering)
    let suggestions = completer.complete("$actor.n", 8);

    assert_eq!(1, suggestions.len());

    let expected: Vec<String> = vec!["name".into()];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for $env
    let suggestions = completer.complete("$env.", 5);

    assert_eq!(2, suggestions.len());

    let expected: Vec<String> = vec!["PWD".into(), "TEST".into()];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for $env
    let suggestions = completer.complete("$env.T", 6);

    assert_eq!(1, suggestions.len());

    let expected: Vec<String> = vec!["TEST".into()];

    // Match results
    match_suggestions(expected, suggestions);
}

#[test]
fn alias_of_command_and_flags() {
    let (dir, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls -l"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let suggestions = completer.complete("ll t", 4);
    #[cfg(windows)]
    let expected_paths: Vec<String> = vec!["test_a\\".to_string(), "test_b\\".to_string()];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec!["test_a/".to_string(), "test_b/".to_string()];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn alias_of_basic_command() {
    let (dir, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls "#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let suggestions = completer.complete("ll t", 4);
    #[cfg(windows)]
    let expected_paths: Vec<String> = vec!["test_a\\".to_string(), "test_b\\".to_string()];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec!["test_a/".to_string(), "test_b/".to_string()];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn alias_of_another_alias() {
    let (dir, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls -la"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir.clone()).is_ok());
    // Create the second alias
    let alias = r#"alias lf = ll -f"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let suggestions = completer.complete("lf t", 4);
    #[cfg(windows)]
    let expected_paths: Vec<String> = vec!["test_a\\".to_string(), "test_b\\".to_string()];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec!["test_a/".to_string(), "test_b/".to_string()];

    match_suggestions(expected_paths, suggestions)
}

fn run_external_completion(block: &str, input: &str) -> Vec<Suggestion> {
    // Create a new engine
    let (dir, _, mut engine_state, mut stack) = new_engine();
    let (_, delta) = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        let (block, err) = parse(&mut working_set, None, block.as_bytes(), false, &[]);
        assert!(err.is_none());

        (block, working_set.render())
    };

    assert!(engine_state.merge_delta(delta).is_ok());

    // Merge environment into the permanent state
    assert!(engine_state.merge_env(&mut stack, &dir).is_ok());

    let latest_block_id = engine_state.num_blocks() - 1;

    // Change config adding the external completer
    let mut config = engine_state.get_config().clone();
    config.external_completer = Some(latest_block_id);
    engine_state.set_config(&config);

    // Instatiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine_state), stack);

    completer.complete(input, input.len())
}

#[test]
fn unknown_command_completion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "thiscommanddoesnotexist ";
    let suggestions = completer.complete(target_dir, target_dir.len());

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

#[rstest]
fn flagcompletion_triggers_after_cursor(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst -h", 5);
    let expected: Vec<String> = vec!["--help".into(), "--mod".into(), "-h".into(), "-s".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn customcompletion_triggers_after_cursor(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-command c", 11);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn customcompletion_triggers_after_cursor_piped(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-command c | ls", 11);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn flagcompletion_triggers_after_cursor_piped(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst -h | ls", 5);
    let expected: Vec<String> = vec!["--help".into(), "--mod".into(), "-h".into(), "-s".into()];
    match_suggestions(expected, suggestions);
}

#[test]
fn filecompletions_triggers_after_cursor() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let suggestions = completer.complete("cp   test_c", 3);

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

    match_suggestions(expected_paths, suggestions);
}
