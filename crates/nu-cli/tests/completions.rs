pub mod support;

use nu_cli::NuCompleter;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{debugger::WithoutDebug, engine::StateWorkingSet, PipelineData};
use reedline::{Completer, Suggestion};
use rstest::{fixture, rstest};
use std::path::PathBuf;
use support::{
    completions_helpers::{new_partial_engine, new_quote_engine},
    file, folder, match_suggestions, new_engine,
};

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

#[fixture]
fn extern_completer() -> NuCompleter {
    // Create a new engine
    let (dir, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = r#"
        def animals [] { [ "cat", "dog", "eel" ] }
        extern spam [
            animal: string@animals
            --foo (-f): string@animals
            -b: string@animals
        ]
    "#;
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    // Instantiate a new completer
    NuCompleter::new(std::sync::Arc::new(engine), stack)
}

#[fixture]
fn custom_completer() -> NuCompleter {
    // Create a new engine
    let (dir, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = r#"
        let external_completer = {|spans|
            $spans
        }

        $env.config.completions.external = {
            enable: true
            max_results: 100
            completer: $external_completer
        }
    "#;
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

    assert_eq!(8, suggestions.len());
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

    // Instantiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test source completion
    let completion_str = "source-env ".to_string();
    let suggestions = completer.complete(&completion_str, completion_str.len());

    assert_eq!(2, suggestions.len());
    assert_eq!("custom_completion.nu", suggestions.first().unwrap().value);
    #[cfg(windows)]
    assert_eq!("directory_completion\\", suggestions.get(1).unwrap().value);
    #[cfg(not(windows))]
    assert_eq!("directory_completion/", suggestions.get(1).unwrap().value);

    // Test use completion
    let completion_str = "use ".to_string();
    let suggestions = completer.complete(&completion_str, completion_str.len());

    assert_eq!(2, suggestions.len());
    assert_eq!("custom_completion.nu", suggestions.first().unwrap().value);
    #[cfg(windows)]
    assert_eq!("directory_completion\\", suggestions.get(1).unwrap().value);
    #[cfg(not(windows))]
    assert_eq!("directory_completion/", suggestions.get(1).unwrap().value);

    // Test overlay use completion
    let completion_str = "overlay use ".to_string();
    let suggestions = completer.complete(&completion_str, completion_str.len());

    assert_eq!(2, suggestions.len());
    assert_eq!("custom_completion.nu", suggestions.first().unwrap().value);
    #[cfg(windows)]
    assert_eq!("directory_completion\\", suggestions.get(1).unwrap().value);
    #[cfg(not(windows))]
    assert_eq!("directory_completion/", suggestions.get(1).unwrap().value);
}

#[test]
#[ignore]
fn external_completer_trailing_space() {
    // https://github.com/nushell/nushell/issues/6378
    let block = "{|spans| $spans}";
    let input = "gh alias ".to_string();

    let suggestions = run_external_completion(block, &input);
    assert_eq!(3, suggestions.len());
    assert_eq!("gh", suggestions.first().unwrap().value);
    assert_eq!("alias", suggestions.get(1).unwrap().value);
    assert_eq!("", suggestions.get(2).unwrap().value);
}

#[test]
fn external_completer_no_trailing_space() {
    let block = "{|spans| $spans}";
    let input = "gh alias".to_string();

    let suggestions = run_external_completion(block, &input);
    assert_eq!(2, suggestions.len());
    assert_eq!("gh", suggestions.first().unwrap().value);
    assert_eq!("alias", suggestions.get(1).unwrap().value);
}

#[test]
fn external_completer_pass_flags() {
    let block = "{|spans| $spans}";
    let input = "gh api --".to_string();

    let suggestions = run_external_completion(block, &input);
    assert_eq!(3, suggestions.len());
    assert_eq!("gh", suggestions.first().unwrap().value);
    assert_eq!("api", suggestions.get(1).unwrap().value);
    assert_eq!("--", suggestions.get(2).unwrap().value);
}

#[test]
fn file_completions() {
    // Create a new engine
    let (dir, dir_str, engine, stack) = new_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for the current folder
    let target_dir = format!("cp {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![
        folder(dir.join("another")),
        file(dir.join("custom_completion.nu")),
        folder(dir.join("directory_completion")),
        file(dir.join("nushell")),
        folder(dir.join("test_a")),
        folder(dir.join("test_b")),
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

    // Test completions for hidden files
    let target_dir = format!("ls {}/.", folder(dir.join(".hidden_folder")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    let expected_paths: Vec<String> =
        vec![file(dir.join(".hidden_folder").join(".hidden_subfile"))];

    // Match the results
    match_suggestions(expected_paths, suggestions);
}

#[test]
fn partial_completions() {
    // Create a new engine
    let (dir, _, engine, stack) = new_partial_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for a folder's name
    let target_dir = format!("cd {}", file(dir.join("pa")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![
        folder(dir.join("partial_a")),
        folder(dir.join("partial_b")),
        folder(dir.join("partial_c")),
    ];

    // Match the results
    match_suggestions(expected_paths, suggestions);

    // Test completions for the files whose name begin with "h"
    // and are present under directories whose names begin with "pa"
    let dir_str = file(dir.join("pa").join("h"));
    let target_dir = format!("cp {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![
        file(dir.join("partial_a").join("hello")),
        file(dir.join("partial_a").join("hola")),
        file(dir.join("partial_b").join("hello_b")),
        file(dir.join("partial_b").join("hi_b")),
        file(dir.join("partial_c").join("hello_c")),
    ];

    // Match the results
    match_suggestions(expected_paths, suggestions);

    // Test completion for all files under directories whose names begin with "pa"
    let dir_str = folder(dir.join("pa"));
    let target_dir = format!("ls {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![
        file(dir.join("partial_a").join("anotherfile")),
        file(dir.join("partial_a").join("hello")),
        file(dir.join("partial_a").join("hola")),
        file(dir.join("partial_b").join("hello_b")),
        file(dir.join("partial_b").join("hi_b")),
        file(dir.join("partial_c").join("hello_c")),
    ];

    // Match the results
    match_suggestions(expected_paths, suggestions);

    // Test completion for a single file
    let dir_str = file(dir.join("fi").join("so"));
    let target_dir = format!("rm {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![file(dir.join("final_partial").join("somefile"))];

    // Match the results
    match_suggestions(expected_paths, suggestions);

    // Test completion where there is a sneaky `..` in the path
    let dir_str = file(dir.join("par").join("..").join("fi").join("so"));
    let target_dir = format!("rm {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![file(dir.join("final_partial").join("somefile"))];

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
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion\\".to_string(),
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion/".to_string(),
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
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
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion\\".to_string(),
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion/".to_string(),
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
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
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion\\".to_string(),
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion/".to_string(),
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
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
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion\\".to_string(),
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion/".to_string(),
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
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
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion\\".to_string(),
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion/".to_string(),
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
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
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion\\".to_string(),
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion/".to_string(),
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
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
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion\\".to_string(),
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion/".to_string(),
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder/".to_string(),
    ];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn file_completion_quoted() {
    let (_, _, engine, stack) = new_quote_engine();

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    let target_dir = "open ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    let expected_paths: Vec<String> = vec![
        "\'[a] bc.txt\'".to_string(),
        "`--help`".to_string(),
        "`-42`".to_string(),
        "`-inf`".to_string(),
        "`4.2`".to_string(),
        "`te st.txt`".to_string(),
        "`te#st.txt`".to_string(),
        "`te'st.txt`".to_string(),
        "`te(st).txt`".to_string(),
        format!("`{}`", folder("test dir".into())),
    ];

    match_suggestions(expected_paths, suggestions);

    let dir: PathBuf = "test dir".into();
    let target_dir = format!("open '{}'", folder(dir.clone()));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    let expected_paths: Vec<String> = vec![
        format!("`{}`", file(dir.join("double quote"))),
        format!("`{}`", file(dir.join("single quote"))),
    ];

    match_suggestions(expected_paths, suggestions)
}

#[test]
fn flag_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);
    // Test completions for the 'ls' flags
    let suggestions = completer.complete("ls -", 4);

    assert_eq!(16, suggestions.len());

    let expected: Vec<String> = vec![
        "--all".into(),
        "--directory".into(),
        "--du".into(),
        "--full-paths".into(),
        "--help".into(),
        "--long".into(),
        "--mime-type".into(),
        "--short-names".into(),
        "-D".into(),
        "-a".into(),
        "-d".into(),
        "-f".into(),
        "-h".into(),
        "-l".into(),
        "-m".into(),
        "-s".into(),
    ];

    // Match results
    match_suggestions(expected, suggestions);
}

#[test]
fn folder_with_directorycompletions() {
    // Create a new engine
    let (dir, dir_str, engine, stack) = new_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for the current folder
    let target_dir = format!("cd {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<String> = vec![
        folder(dir.join("another")),
        folder(dir.join("directory_completion")),
        folder(dir.join("test_a")),
        folder(dir.join("test_b")),
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

    // Instantiate a new completer
    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Test completions for $nu
    let suggestions = completer.complete("$nu.", 4);

    assert_eq!(15, suggestions.len());

    let expected: Vec<String> = vec![
        "config-path".into(),
        "current-exe".into(),
        "default-config-dir".into(),
        "env-path".into(),
        "history-enabled".into(),
        "history-path".into(),
        "home-path".into(),
        "is-interactive".into(),
        "is-login".into(),
        "loginshell-path".into(),
        "os-info".into(),
        "pid".into(),
        "plugin-path".into(),
        "startup-time".into(),
        "temp-path".into(),
    ];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for $nu.h (filter)
    let suggestions = completer.complete("$nu.h", 5);

    assert_eq!(3, suggestions.len());

    let expected: Vec<String> = vec![
        "history-enabled".into(),
        "history-path".into(),
        "home-path".into(),
    ];

    // Match results
    match_suggestions(expected, suggestions);

    // Test completions for $nu.os-info
    let suggestions = completer.complete("$nu.os-info.", 12);
    assert_eq!(4, suggestions.len());
    let expected: Vec<String> = vec![
        "arch".into(),
        "family".into(),
        "kernel_version".into(),
        "name".into(),
    ];
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

    assert_eq!(3, suggestions.len());

    #[cfg(windows)]
    let expected: Vec<String> = vec!["PWD".into(), "Path".into(), "TEST".into()];
    #[cfg(not(windows))]
    let expected: Vec<String> = vec!["PATH".into(), "PWD".into(), "TEST".into()];

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

fn run_external_completion(completer: &str, input: &str) -> Vec<Suggestion> {
    let completer = format!("$env.config.completions.external.completer = {completer}");

    // Create a new engine
    let (dir, _, mut engine_state, mut stack) = new_engine();
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        let block = parse(&mut working_set, None, completer.as_bytes(), false);
        assert!(working_set.parse_errors.is_empty());

        (block, working_set.render())
    };

    assert!(engine_state.merge_delta(delta).is_ok());

    assert!(
        eval_block::<WithoutDebug>(&engine_state, &mut stack, &block, PipelineData::Empty).is_ok()
    );

    // Merge environment into the permanent state
    assert!(engine_state.merge_env(&mut stack, &dir).is_ok());

    // Instantiate a new completer
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
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion\\".to_string(),
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion/".to_string(),
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
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
        "another\\".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion\\".to_string(),
        "nushell".to_string(),
        "test_a\\".to_string(),
        "test_b\\".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder\\".to_string(),
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<String> = vec![
        "another/".to_string(),
        "custom_completion.nu".to_string(),
        "directory_completion/".to_string(),
        "nushell".to_string(),
        "test_a/".to_string(),
        "test_b/".to_string(),
        ".hidden_file".to_string(),
        ".hidden_folder/".to_string(),
    ];

    match_suggestions(expected_paths, suggestions);
}

#[rstest]
fn extern_custom_completion_positional(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam ", 5);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn extern_custom_completion_long_flag_1(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam --foo=", 11);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn extern_custom_completion_long_flag_2(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam --foo ", 11);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn extern_custom_completion_long_flag_short(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam -f ", 8);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn extern_custom_completion_short_flag(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam -b ", 8);
    let expected: Vec<String> = vec!["cat".into(), "dog".into(), "eel".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn extern_complete_flags(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam -", 6);
    let expected: Vec<String> = vec!["--foo".into(), "-b".into(), "-f".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn custom_completer_triggers_cursor_before_word(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("cmd foo  bar", 8);
    let expected: Vec<String> = vec!["cmd".into(), "foo".into(), "".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn custom_completer_triggers_cursor_on_word_left_boundary(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("cmd foo bar", 8);
    let expected: Vec<String> = vec!["cmd".into(), "foo".into(), "".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn custom_completer_triggers_cursor_next_to_word(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("cmd foo bar", 11);
    let expected: Vec<String> = vec!["cmd".into(), "foo".into(), "bar".into()];
    match_suggestions(expected, suggestions);
}

#[rstest]
fn custom_completer_triggers_cursor_after_word(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("cmd foo bar ", 12);
    let expected: Vec<String> = vec!["cmd".into(), "foo".into(), "bar".into(), "".into()];
    match_suggestions(expected, suggestions);
}

#[ignore = "was reverted, still needs fixing"]
#[rstest]
fn alias_offset_bug_7648() {
    let (dir, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ea = ^$env.EDITOR /tmp/test.s"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Issue #7648
    // Nushell crashes when an alias name is shorter than the alias command
    // and the alias command is a external command
    // This happens because of offset is not correct.
    // This crashes before PR #7779
    let _suggestions = completer.complete("e", 1);
}

#[ignore = "was reverted, still needs fixing"]
#[rstest]
fn alias_offset_bug_7754() {
    let (dir, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls -l"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack, dir).is_ok());

    let mut completer = NuCompleter::new(std::sync::Arc::new(engine), stack);

    // Issue #7754
    // Nushell crashes when an alias name is shorter than the alias command
    // and the alias command contains pipes.
    // This crashes before PR #7756
    let _suggestions = completer.complete("ll -a | c", 9);
}

#[test]
fn get_path_env_var_8003() {
    // Create a new engine
    let (_, _, engine, _) = new_engine();
    // Get the path env var in a platform agnostic way
    let the_path = engine.get_path_env_var();
    // Make sure it's not empty
    assert!(the_path.is_some());
}
