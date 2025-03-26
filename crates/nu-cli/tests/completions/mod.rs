pub mod support;

use std::{
    fs::{read_dir, FileType, ReadDir},
    path::{PathBuf, MAIN_SEPARATOR},
    sync::Arc,
};

use nu_cli::NuCompleter;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_path::expand_tilde;
use nu_protocol::{debugger::WithoutDebug, engine::StateWorkingSet, PipelineData};
use reedline::{Completer, Suggestion};
use rstest::{fixture, rstest};
use support::{
    completions_helpers::{
        new_dotnu_engine, new_external_engine, new_partial_engine, new_quote_engine,
    },
    file, folder, match_suggestions, match_suggestions_by_string, new_engine,
};

// Match a list of suggestions with the content of a directory.
// This helper is for DotNutCompletion, so actually it only retrieves
// *.nu files and subdirectories.
pub fn match_dir_content_for_dotnu(dir: ReadDir, suggestions: &[Suggestion]) {
    let actual_dir_entries: Vec<_> = dir.filter_map(|c| c.ok()).collect();
    let type_name_pairs: Vec<(FileType, String)> = actual_dir_entries
        .into_iter()
        .filter_map(|t| t.file_type().ok().zip(t.file_name().into_string().ok()))
        .collect();
    let mut simple_dir_entries: Vec<&str> = type_name_pairs
        .iter()
        .filter_map(|(t, n)| {
            if t.is_dir() || n.ends_with(".nu") {
                Some(n.as_str())
            } else {
                None
            }
        })
        .collect();
    simple_dir_entries.sort();
    let mut pure_suggestions: Vec<&str> = suggestions
        .iter()
        .map(|s| {
            // The file names in suggestions contain some extra characters,
            // we clean them to compare more exactly with read_dir result.
            s.value
                .as_str()
                .trim_end_matches('`')
                .trim_end_matches('/')
                .trim_end_matches('\\')
                .trim_start_matches('`')
                .trim_start_matches("~/")
                .trim_start_matches("~\\")
        })
        .collect();
    pure_suggestions.sort();
    assert_eq!(simple_dir_entries, pure_suggestions);
}

#[fixture]
fn completer() -> NuCompleter {
    // Create a new engine
    let (_, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = "def tst [--mod -s] {}";
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack).is_ok());

    // Instantiate a new completer
    NuCompleter::new(Arc::new(engine), Arc::new(stack))
}

#[fixture]
fn completer_strings() -> NuCompleter {
    // Create a new engine
    let (_, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = r#"def animals [] { ["cat", "dog", "eel" ] }
    def my-command [animal: string@animals] { print $animal }"#;
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack).is_ok());

    // Instantiate a new completer
    NuCompleter::new(Arc::new(engine), Arc::new(stack))
}

#[fixture]
fn extern_completer() -> NuCompleter {
    // Create a new engine
    let (_, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = r#"
        def animals [] { [ "cat", "dog", "eel" ] }
        extern spam [
            animal: string@animals
            --foo (-f): string@animals
            -b: string@animals
        ]
    "#;
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack).is_ok());

    // Instantiate a new completer
    NuCompleter::new(Arc::new(engine), Arc::new(stack))
}

fn custom_completer_with_options(
    global_opts: &str,
    completer_opts: &str,
    completions: &[&str],
) -> NuCompleter {
    let (_, _, mut engine, mut stack) = new_engine();
    let command = format!(
        r#"
        {}
        def comp [] {{
            {{ completions: [{}], options: {{ {} }} }}
        }}
        def my-command [arg: string@comp] {{}}"#,
        global_opts,
        completions
            .iter()
            .map(|comp| format!("'{}'", comp))
            .collect::<Vec<_>>()
            .join(", "),
        completer_opts,
    );
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());

    NuCompleter::new(Arc::new(engine), Arc::new(stack))
}

#[fixture]
fn custom_completer() -> NuCompleter {
    // Create a new engine
    let (_, _, mut engine, mut stack) = new_engine();

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
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack).is_ok());

    // Instantiate a new completer
    NuCompleter::new(Arc::new(engine), Arc::new(stack))
}

/// Use fuzzy completions but sort in alphabetical order
#[fixture]
fn fuzzy_alpha_sort_completer() -> NuCompleter {
    // Create a new engine
    let (_, _, mut engine, mut stack) = new_engine();

    let config = r#"
        $env.config.completions.algorithm = "fuzzy"
        $env.config.completions.sort = "alphabetical"
    "#;
    assert!(support::merge_input(config.as_bytes(), &mut engine, &mut stack).is_ok());

    // Instantiate a new completer
    NuCompleter::new(Arc::new(engine), Arc::new(stack))
}

#[test]
fn variables_dollar_sign_with_variablecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "$ ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    assert_eq!(9, suggestions.len());
}

#[rstest]
fn variables_double_dash_argument_with_flagcompletion(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst --", 6);
    let expected: Vec<_> = vec!["--help", "--mod"];
    // dbg!(&expected, &suggestions);
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn variables_single_dash_argument_with_flagcompletion(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst -", 5);
    let expected: Vec<_> = vec!["--help", "--mod", "-h", "-s"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn variables_command_with_commandcompletion(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-c ", 4);
    let expected: Vec<_> = vec!["my-command"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn variables_subcommands_with_customcompletion(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-command ", 11);
    let expected: Vec<_> = vec!["cat", "dog", "eel"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn variables_customcompletion_subcommands_with_customcompletion_2(
    mut completer_strings: NuCompleter,
) {
    let suggestions = completer_strings.complete("my-command ", 11);
    let expected: Vec<_> = vec!["cat", "dog", "eel"];
    match_suggestions(&expected, &suggestions);
}

/// $env.config should be overridden by the custom completer's options
#[test]
fn customcompletions_override_options() {
    let mut completer = custom_completer_with_options(
        r#"$env.config.completions.algorithm = "fuzzy"
           $env.config.completions.case_sensitive = false"#,
        r#"completion_algorithm: "prefix",
           positional: false,
           case_sensitive: true,
           sort: true"#,
        &["Foo Abcdef", "Abcdef", "Acd Bar"],
    );

    // positional: false should make it do substring matching
    // sort: true should force sorting
    let expected: Vec<_> = vec!["Abcdef", "Foo Abcdef"];
    let suggestions = completer.complete("my-command Abcd", 15);
    match_suggestions(&expected, &suggestions);

    // Custom options should make case-sensitive
    let suggestions = completer.complete("my-command aBcD", 15);
    assert!(suggestions.is_empty());
}

/// $env.config should be inherited by the custom completer's options
#[test]
fn customcompletions_inherit_options() {
    let mut completer = custom_completer_with_options(
        r#"$env.config.completions.algorithm = "fuzzy"
           $env.config.completions.case_sensitive = false"#,
        "",
        &["Foo Abcdef", "Abcdef", "Acd Bar"],
    );

    // Make sure matching is fuzzy
    let suggestions = completer.complete("my-command Acd", 14);
    let expected: Vec<_> = vec!["Acd Bar", "Abcdef", "Foo Abcdef"];
    match_suggestions(&expected, &suggestions);

    // Custom options should make matching case insensitive
    let suggestions = completer.complete("my-command acd", 14);
    match_suggestions(&expected, &suggestions);
}

#[test]
fn customcompletions_no_sort() {
    let mut completer = custom_completer_with_options(
        "",
        r#"completion_algorithm: "fuzzy",
           sort: false"#,
        &["zzzfoo", "foo", "not matched", "abcfoo"],
    );
    let suggestions = completer.complete("my-command foo", 14);
    let expected: Vec<_> = vec!["zzzfoo", "foo", "abcfoo"];
    match_suggestions(&expected, &suggestions);
}

/// Fallback to file completions if custom completer returns null
#[test]
fn customcompletions_fallback() {
    let (_, _, mut engine, mut stack) = new_engine();
    let command = r#"
        def comp [] { null }
        def my-command [arg: string@comp] {}"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    let completion_str = "my-command test";
    let suggestions = completer.complete(completion_str, completion_str.len());
    let expected = [folder("test_a"), file("test_a_symlink"), folder("test_b")];
    match_suggestions_by_string(&expected, &suggestions);
}

/// Custom function arguments mixed with subcommands
#[test]
fn custom_arguments_and_subcommands() {
    let (_, _, mut engine, mut stack) = new_engine();
    let command = r#"
        def foo [i: directory] {}
        def "foo test bar" [] {}"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    let completion_str = "foo test";
    let suggestions = completer.complete(completion_str, completion_str.len());
    // including both subcommand and directory completions
    let expected = [
        "foo test bar".into(),
        folder("test_a"),
        file("test_a_symlink"),
        folder("test_b"),
    ];
    match_suggestions_by_string(&expected, &suggestions);
}

/// Custom function flags mixed with subcommands
#[test]
fn custom_flags_and_subcommands() {
    let (_, _, mut engine, mut stack) = new_engine();
    let command = r#"
        def foo [--test: directory] {}
        def "foo --test bar" [] {}"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    let completion_str = "foo --test";
    let suggestions = completer.complete(completion_str, completion_str.len());
    // including both flag and directory completions
    let expected: Vec<_> = vec!["foo --test bar", "--test"];
    match_suggestions(&expected, &suggestions);
}

/// If argument type is something like int/string, complete only subcommands
#[test]
fn custom_arguments_vs_subcommands() {
    let (_, _, mut engine, mut stack) = new_engine();
    let command = r#"
        def foo [i: string] {}
        def "foo test bar" [] {}"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    let completion_str = "foo test";
    let suggestions = completer.complete(completion_str, completion_str.len());
    // including only subcommand completions
    let expected: Vec<_> = vec!["foo test bar"];
    match_suggestions(&expected, &suggestions);
}

#[test]
fn custom_completions_defined_inline() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    let completion_str = "def animals [] { [cat dog] }
export def say [
  animal: string@animals
] { }; say ";
    let suggestions = completer.complete(completion_str, completion_str.len());
    // including only subcommand completions
    let expected: Vec<_> = vec!["cat", "dog"];
    match_suggestions(&expected, &suggestions);
}

/// External command only if starts with `^`
#[test]
fn external_commands_only() {
    let engine = new_external_engine();
    let mut completer = NuCompleter::new(
        Arc::new(engine),
        Arc::new(nu_protocol::engine::Stack::new()),
    );
    let completion_str = "ls; ^sleep";
    let suggestions = completer.complete(completion_str, completion_str.len());
    #[cfg(windows)]
    let expected: Vec<_> = vec!["sleep.exe"];
    #[cfg(not(windows))]
    let expected: Vec<_> = vec!["sleep"];
    match_suggestions(&expected, &suggestions);

    let completion_str = "sleep";
    let suggestions = completer.complete(completion_str, completion_str.len());
    #[cfg(windows)]
    let expected: Vec<_> = vec!["sleep", "sleep.exe"];
    #[cfg(not(windows))]
    let expected: Vec<_> = vec!["sleep", "^sleep"];
    match_suggestions(&expected, &suggestions);
}

/// Which completes both internals and externals
#[test]
fn which_command_completions() {
    let engine = new_external_engine();
    let mut completer = NuCompleter::new(
        Arc::new(engine),
        Arc::new(nu_protocol::engine::Stack::new()),
    );
    // flags
    let completion_str = "which --all";
    let suggestions = completer.complete(completion_str, completion_str.len());
    let expected: Vec<_> = vec!["--all"];
    match_suggestions(&expected, &suggestions);
    // commands
    let completion_str = "which sleep";
    let suggestions = completer.complete(completion_str, completion_str.len());
    #[cfg(windows)]
    let expected: Vec<_> = vec!["sleep", "sleep.exe"];
    #[cfg(not(windows))]
    let expected: Vec<_> = vec!["sleep", "^sleep"];
    match_suggestions(&expected, &suggestions);
}

/// Suppress completions for invalid values
#[test]
fn customcompletions_invalid() {
    let (_, _, mut engine, mut stack) = new_engine();
    let command = r#"
        def comp [] { 123 }
        def my-command [arg: string@comp] {}"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let completion_str = "my-command foo";
    let suggestions = completer.complete(completion_str, completion_str.len());
    assert!(suggestions.is_empty());
}

#[test]
fn dont_use_dotnu_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_dotnu_engine();
    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    // Test nested nu script
    let completion_str = "go work use `./dir_module/";
    let suggestions = completer.complete(completion_str, completion_str.len());

    // including a plaintext file
    let expected: Vec<_> = vec![
        "./dir_module/mod.nu",
        "./dir_module/plain.txt",
        "`./dir_module/sub module/`",
    ];
    match_suggestions(&expected, &suggestions);
}

#[test]
fn dotnu_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_dotnu_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Flags should still be working
    let completion_str = "overlay use --";
    let suggestions = completer.complete(completion_str, completion_str.len());

    match_suggestions(&vec!["--help", "--prefix", "--reload"], &suggestions);

    // Test nested nu script
    #[cfg(windows)]
    let completion_str = "use `.\\dir_module\\";
    #[cfg(not(windows))]
    let completion_str = "use `./dir_module/";
    let suggestions = completer.complete(completion_str, completion_str.len());

    match_suggestions(
        &vec![
            "mod.nu",
            #[cfg(windows)]
            "sub module\\`",
            #[cfg(not(windows))]
            "sub module/`",
        ],
        &suggestions,
    );

    // Test nested nu script, with ending '`'
    #[cfg(windows)]
    let completion_str = "use `.\\dir_module\\sub module\\`";
    #[cfg(not(windows))]
    let completion_str = "use `./dir_module/sub module/`";
    let suggestions = completer.complete(completion_str, completion_str.len());

    match_suggestions(&vec!["sub.nu`"], &suggestions);

    let expected = vec![
        "asdf.nu",
        "bar.nu",
        "bat.nu",
        "baz.nu",
        #[cfg(windows)]
        "dir_module\\",
        #[cfg(not(windows))]
        "dir_module/",
        "foo.nu",
        #[cfg(windows)]
        "lib-dir1\\",
        #[cfg(not(windows))]
        "lib-dir1/",
        #[cfg(windows)]
        "lib-dir2\\",
        #[cfg(not(windows))]
        "lib-dir2/",
        #[cfg(windows)]
        "lib-dir3\\",
        #[cfg(not(windows))]
        "lib-dir3/",
        "spam.nu",
        "xyzzy.nu",
    ];

    // Test source completion
    let completion_str = "source-env ";
    let suggestions = completer.complete(completion_str, completion_str.len());

    match_suggestions(&expected, &suggestions);

    // Test use completion
    let completion_str = "use ";
    let suggestions = completer.complete(completion_str, completion_str.len());

    match_suggestions(&expected, &suggestions);

    // Test overlay use completion
    let completion_str = "overlay use ";
    let suggestions = completer.complete(completion_str, completion_str.len());

    match_suggestions(&expected, &suggestions);

    // Test special paths
    #[cfg(windows)]
    {
        let completion_str = "use \\";
        let dir_content = read_dir("\\").unwrap();
        let suggestions = completer.complete(completion_str, completion_str.len());
        match_dir_content_for_dotnu(dir_content, &suggestions);
    }

    let completion_str = "use /";
    let suggestions = completer.complete(completion_str, completion_str.len());
    let dir_content = read_dir("/").unwrap();
    match_dir_content_for_dotnu(dir_content, &suggestions);

    let completion_str = "use ~";
    let dir_content = read_dir(expand_tilde("~")).unwrap();
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_dir_content_for_dotnu(dir_content, &suggestions);
}

#[test]
fn dotnu_completions_const_nu_lib_dirs() {
    let (_, _, engine, stack) = new_dotnu_engine();
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // file in `lib-dir1/`, set by `const NU_LIB_DIRS`
    let completion_str = "use xyzz";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&vec!["xyzzy.nu"], &suggestions);

    // file in `lib-dir2/`, set by `$env.NU_LIB_DIRS`
    let completion_str = "use asdf";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&vec!["asdf.nu"], &suggestions);

    // file in `lib-dir3/`, set by both, should not replicate
    let completion_str = "use spam";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&vec!["spam.nu"], &suggestions);

    // if `./` specified by user, file in `lib-dir*` should be ignored
    #[cfg(windows)]
    {
        let completion_str = "use .\\asdf";
        let suggestions = completer.complete(completion_str, completion_str.len());
        assert!(suggestions.is_empty());
    }
    let completion_str = "use ./asdf";
    let suggestions = completer.complete(completion_str, completion_str.len());
    assert!(suggestions.is_empty());
}

#[test]
#[ignore]
fn external_completer_trailing_space() {
    // https://github.com/nushell/nushell/issues/6378
    let block = "{|spans| $spans}";
    let input = "gh alias ";

    let suggestions = run_external_completion(block, input);
    assert_eq!(3, suggestions.len());
    assert_eq!("gh", suggestions.first().unwrap().value);
    assert_eq!("alias", suggestions.get(1).unwrap().value);
    assert_eq!("", suggestions.get(2).unwrap().value);
}

#[test]
fn external_completer_no_trailing_space() {
    let block = "{|spans| $spans}";
    let input = "gh alias";

    let suggestions = run_external_completion(block, input);
    assert_eq!(2, suggestions.len());
    assert_eq!("gh", suggestions.first().unwrap().value);
    assert_eq!("alias", suggestions.get(1).unwrap().value);
}

#[test]
fn external_completer_pass_flags() {
    let block = "{|spans| $spans}";
    let input = "gh api --";

    let suggestions = run_external_completion(block, input);
    assert_eq!(3, suggestions.len());
    assert_eq!("gh", suggestions.first().unwrap().value);
    assert_eq!("api", suggestions.get(1).unwrap().value);
    assert_eq!("--", suggestions.get(2).unwrap().value);
}

/// Fallback to file completions when external completer returns null
#[test]
fn external_completer_fallback() {
    let block = "{|spans| null}";
    let input = "foo test";

    let expected = [folder("test_a"), file("test_a_symlink"), folder("test_b")];
    let suggestions = run_external_completion(block, input);
    match_suggestions_by_string(&expected, &suggestions);
}

/// Fallback to external completions for flags of `sudo`
#[test]
fn external_completer_sudo() {
    let block = "{|spans| ['--background']}";
    let input = "sudo --back";

    let expected = vec!["--background"];
    let suggestions = run_external_completion(block, input);
    match_suggestions(&expected, &suggestions);
}

/// Suppress completions when external completer returns invalid value
#[test]
fn external_completer_invalid() {
    let block = "{|spans| 123}";
    let input = "foo ";

    let suggestions = run_external_completion(block, input);
    assert!(suggestions.is_empty());
}

#[test]
fn file_completions() {
    // Create a new engine
    let (dir, dir_str, engine, stack) = new_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Test completions for the current folder
    let target_dir = format!("cp {dir_str}{MAIN_SEPARATOR}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        folder(dir.join("another")),
        file(dir.join("custom_completion.nu")),
        folder(dir.join("directory_completion")),
        file(dir.join("nushell")),
        folder(dir.join("test_a")),
        file(dir.join("test_a_symlink")),
        folder(dir.join("test_b")),
        file(dir.join(".hidden_file")),
        folder(dir.join(".hidden_folder")),
    ];

    #[cfg(windows)]
    {
        let separator = '/';
        let target_dir = format!("cp {dir_str}{separator}");
        let slash_suggestions = completer.complete(&target_dir, target_dir.len());

        let expected_slash_paths: Vec<_> = expected_paths
            .iter()
            .map(|s| s.replace('\\', "/"))
            .collect();

        match_suggestions_by_string(&expected_slash_paths, &slash_suggestions);
    }

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completions for the current folder even with parts before the autocomplet
    let target_dir = format!("cp somefile.txt {dir_str}{MAIN_SEPARATOR}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        folder(dir.join("another")),
        file(dir.join("custom_completion.nu")),
        folder(dir.join("directory_completion")),
        file(dir.join("nushell")),
        folder(dir.join("test_a")),
        file(dir.join("test_a_symlink")),
        folder(dir.join("test_b")),
        file(dir.join(".hidden_file")),
        folder(dir.join(".hidden_folder")),
    ];

    #[cfg(windows)]
    {
        let separator = '/';
        let target_dir = format!("cp somefile.txt {dir_str}{separator}");
        let slash_suggestions = completer.complete(&target_dir, target_dir.len());

        let expected_slash_paths: Vec<_> = expected_paths
            .iter()
            .map(|s| s.replace('\\', "/"))
            .collect();

        match_suggestions_by_string(&expected_slash_paths, &slash_suggestions);
    }

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completions for a file
    let target_dir = format!("cp {}", folder(dir.join("another")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [file(dir.join("another").join("newfile"))];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completions for hidden files
    let target_dir = format!("ls {}", file(dir.join(".hidden_folder").join(".")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    let expected_paths = [file(dir.join(".hidden_folder").join(".hidden_subfile"))];

    #[cfg(windows)]
    {
        let target_dir = format!("ls {}/.", folder(dir.join(".hidden_folder")));
        let slash_suggestions = completer.complete(&target_dir, target_dir.len());

        let expected_slash: Vec<_> = expected_paths
            .iter()
            .map(|s| s.replace('\\', "/"))
            .collect();

        match_suggestions_by_string(&expected_slash, &slash_suggestions);
    }

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);
}

#[test]
fn custom_command_rest_any_args_file_completions() {
    // Create a new engine
    let (dir, dir_str, mut engine, mut stack) = new_engine();
    let command = r#"def list [ ...args: any ] {}"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Test completions for the current folder
    let target_dir = format!("list {dir_str}{MAIN_SEPARATOR}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        folder(dir.join("another")),
        file(dir.join("custom_completion.nu")),
        folder(dir.join("directory_completion")),
        file(dir.join("nushell")),
        folder(dir.join("test_a")),
        file(dir.join("test_a_symlink")),
        folder(dir.join("test_b")),
        file(dir.join(".hidden_file")),
        folder(dir.join(".hidden_folder")),
    ];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completions for the current folder even with parts before the autocomplet
    let target_dir = format!("list somefile.txt {dir_str}{MAIN_SEPARATOR}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        folder(dir.join("another")),
        file(dir.join("custom_completion.nu")),
        folder(dir.join("directory_completion")),
        file(dir.join("nushell")),
        folder(dir.join("test_a")),
        file(dir.join("test_a_symlink")),
        folder(dir.join("test_b")),
        file(dir.join(".hidden_file")),
        folder(dir.join(".hidden_folder")),
    ];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completions for a file
    let target_dir = format!("list {}", folder(dir.join("another")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [file(dir.join("another").join("newfile"))];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completions for hidden files
    let target_dir = format!("list {}", file(dir.join(".hidden_folder").join(".")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    let expected_paths = [file(dir.join(".hidden_folder").join(".hidden_subfile"))];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);
}

#[cfg(windows)]
#[test]
fn file_completions_with_mixed_separators() {
    // Create a new engine
    let (dir, dir_str, engine, stack) = new_dotnu_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Create Expected values
    let expected_paths: Vec<_> = vec![
        file(dir.join("lib-dir1").join("bar.nu")),
        file(dir.join("lib-dir1").join("baz.nu")),
        file(dir.join("lib-dir1").join("xyzzy.nu")),
    ];
    let expected_slash_paths: Vec<_> = expected_paths
        .iter()
        .map(|s| s.replace(MAIN_SEPARATOR, "/"))
        .collect();

    let target_dir = format!("ls {dir_str}/lib-dir1/");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    match_suggestions_by_string(&expected_slash_paths, &suggestions);

    let target_dir = format!("cp {dir_str}\\lib-dir1/");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    match_suggestions_by_string(&expected_slash_paths, &suggestions);

    let target_dir = format!("ls {dir_str}/lib-dir1\\/");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    match_suggestions_by_string(&expected_slash_paths, &suggestions);

    let target_dir = format!("ls {dir_str}\\lib-dir1\\/");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    match_suggestions_by_string(&expected_slash_paths, &suggestions);

    let target_dir = format!("ls {dir_str}\\lib-dir1\\");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    match_suggestions_by_string(&expected_paths, &suggestions);

    let target_dir = format!("ls {dir_str}/lib-dir1\\");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    match_suggestions_by_string(&expected_paths, &suggestions);

    let target_dir = format!("ls {dir_str}/lib-dir1/\\");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    match_suggestions_by_string(&expected_paths, &suggestions);

    let target_dir = format!("ls {dir_str}\\lib-dir1/\\");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    match_suggestions_by_string(&expected_paths, &suggestions);
}

#[test]
fn partial_completions() {
    // Create a new engine
    let (dir, _, engine, stack) = new_partial_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Test completions for a folder's name
    let target_dir = format!("cd {}", file(dir.join("pa")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        folder(dir.join("partial")),
        folder(dir.join("partial-a")),
        folder(dir.join("partial-b")),
        folder(dir.join("partial-c")),
        format!("`{}`", folder(dir.join("partial-d("))),
    ];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completions for the files whose name begin with "h"
    // and are present under directories whose names begin with "pa"
    let dir_str = file(dir.join("pa").join("h"));
    let target_dir = format!("cp {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        file(dir.join("partial").join("hello.txt")),
        file(dir.join("partial-a").join("have_ext.exe")),
        file(dir.join("partial-a").join("have_ext.txt")),
        file(dir.join("partial-a").join("hello")),
        file(dir.join("partial-a").join("hola")),
        file(dir.join("partial-b").join("hello_b")),
        file(dir.join("partial-b").join("hi_b")),
        file(dir.join("partial-c").join("hello_c")),
    ];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completion for all files under directories whose names begin with "pa"
    let dir_str = folder(dir.join("pa"));
    let target_dir = format!("ls {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        file(dir.join("partial").join("hello.txt")),
        file(dir.join("partial-a").join("anotherfile")),
        file(dir.join("partial-a").join("have_ext.exe")),
        file(dir.join("partial-a").join("have_ext.txt")),
        file(dir.join("partial-a").join("hello")),
        file(dir.join("partial-a").join("hola")),
        file(dir.join("partial-b").join("hello_b")),
        file(dir.join("partial-b").join("hi_b")),
        file(dir.join("partial-c").join("hello_c")),
        format!("`{}`", file(dir.join("partial-d(").join(".gitkeep"))),
    ];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completion for a single file
    let dir_str = file(dir.join("fi").join("so"));
    let target_dir = format!("rm {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [file(dir.join("final_partial").join("somefile"))];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completion where there is a sneaky `..` in the path
    let dir_str = file(dir.join("par").join("..").join("fi").join("so"));
    let target_dir = format!("rm {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        file(
            dir.join("partial")
                .join("..")
                .join("final_partial")
                .join("somefile"),
        ),
        file(
            dir.join("partial-a")
                .join("..")
                .join("final_partial")
                .join("somefile"),
        ),
        file(
            dir.join("partial-b")
                .join("..")
                .join("final_partial")
                .join("somefile"),
        ),
        file(
            dir.join("partial-c")
                .join("..")
                .join("final_partial")
                .join("somefile"),
        ),
        format!(
            "`{}`",
            file(
                dir.join("partial-d(")
                    .join("..")
                    .join("final_partial")
                    .join("somefile"),
            )
        ),
    ];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completion for all files under directories whose names begin with "pa"
    let file_str = file(dir.join("partial-a").join("have"));
    let target_file = format!("rm {file_str}");
    let suggestions = completer.complete(&target_file, target_file.len());

    // Create the expected values
    let expected_paths = [
        file(dir.join("partial-a").join("have_ext.exe")),
        file(dir.join("partial-a").join("have_ext.txt")),
    ];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);

    // Test completion for all files under directories whose names begin with "pa"
    let file_str = file(dir.join("partial-a").join("have_ext."));
    let file_dir = format!("rm {file_str}");
    let suggestions = completer.complete(&file_dir, file_dir.len());

    // Create the expected values
    let expected_paths = [
        file(dir.join("partial-a").join("have_ext.exe")),
        file(dir.join("partial-a").join("have_ext.txt")),
    ];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);
}

#[test]
fn partial_completion_with_dot_expansions() {
    let (dir, _, engine, stack) = new_partial_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let dir_str = file(
        dir.join("par")
            .join("...")
            .join("par")
            .join("fi")
            .join("so"),
    );
    let target_dir = format!("rm {dir_str}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        file(
            dir.join("partial")
                .join("...")
                .join("partial_completions")
                .join("final_partial")
                .join("somefile"),
        ),
        file(
            dir.join("partial-a")
                .join("...")
                .join("partial_completions")
                .join("final_partial")
                .join("somefile"),
        ),
        file(
            dir.join("partial-b")
                .join("...")
                .join("partial_completions")
                .join("final_partial")
                .join("somefile"),
        ),
        file(
            dir.join("partial-c")
                .join("...")
                .join("partial_completions")
                .join("final_partial")
                .join("somefile"),
        ),
        format!(
            "`{}`",
            file(
                dir.join("partial-d(")
                    .join("...")
                    .join("partial_completions")
                    .join("final_partial")
                    .join("somefile"),
            )
        ),
    ];

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);
}

#[test]
fn command_ls_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "ls ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<_> = vec![
        "another\\",
        "custom_completion.nu",
        "directory_completion\\",
        "nushell",
        "test_a\\",
        "test_a_symlink",
        "test_b\\",
        ".hidden_file",
        ".hidden_folder\\",
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec![
        "another/",
        "custom_completion.nu",
        "directory_completion/",
        "nushell",
        "test_a/",
        "test_a_symlink",
        "test_b/",
        ".hidden_file",
        ".hidden_folder/",
    ];

    match_suggestions(&expected_paths, &suggestions);

    let target_dir = "ls custom_completion.";
    let suggestions = completer.complete(target_dir, target_dir.len());

    let expected_paths: Vec<_> = vec!["custom_completion.nu"];

    match_suggestions(&expected_paths, &suggestions);
}

#[test]
fn command_open_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "open ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<_> = vec![
        "another\\",
        "custom_completion.nu",
        "directory_completion\\",
        "nushell",
        "test_a\\",
        "test_a_symlink",
        "test_b\\",
        ".hidden_file",
        ".hidden_folder\\",
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec![
        "another/",
        "custom_completion.nu",
        "directory_completion/",
        "nushell",
        "test_a/",
        "test_a_symlink",
        "test_b/",
        ".hidden_file",
        ".hidden_folder/",
    ];

    match_suggestions(&expected_paths, &suggestions);

    let target_dir = "open custom_completion.";
    let suggestions = completer.complete(target_dir, target_dir.len());

    let expected_paths: Vec<_> = vec!["custom_completion.nu"];

    match_suggestions(&expected_paths, &suggestions);
}

#[test]
fn command_rm_with_globcompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "rm ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<_> = vec![
        "another\\",
        "custom_completion.nu",
        "directory_completion\\",
        "nushell",
        "test_a\\",
        "test_a_symlink",
        "test_b\\",
        ".hidden_file",
        ".hidden_folder\\",
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec![
        "another/",
        "custom_completion.nu",
        "directory_completion/",
        "nushell",
        "test_a/",
        "test_a_symlink",
        "test_b/",
        ".hidden_file",
        ".hidden_folder/",
    ];

    match_suggestions(&expected_paths, &suggestions)
}

#[test]
fn command_cp_with_globcompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "cp ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<_> = vec![
        "another\\",
        "custom_completion.nu",
        "directory_completion\\",
        "nushell",
        "test_a\\",
        "test_a_symlink",
        "test_b\\",
        ".hidden_file",
        ".hidden_folder\\",
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec![
        "another/",
        "custom_completion.nu",
        "directory_completion/",
        "nushell",
        "test_a/",
        "test_a_symlink",
        "test_b/",
        ".hidden_file",
        ".hidden_folder/",
    ];

    match_suggestions(&expected_paths, &suggestions)
}

#[test]
fn command_save_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "save ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<_> = vec![
        "another\\",
        "custom_completion.nu",
        "directory_completion\\",
        "nushell",
        "test_a\\",
        "test_a_symlink",
        "test_b\\",
        ".hidden_file",
        ".hidden_folder\\",
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec![
        "another/",
        "custom_completion.nu",
        "directory_completion/",
        "nushell",
        "test_a/",
        "test_a_symlink",
        "test_b/",
        ".hidden_file",
        ".hidden_folder/",
    ];

    match_suggestions(&expected_paths, &suggestions)
}

#[test]
fn command_touch_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "touch ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<_> = vec![
        "another\\",
        "custom_completion.nu",
        "directory_completion\\",
        "nushell",
        "test_a\\",
        "test_a_symlink",
        "test_b\\",
        ".hidden_file",
        ".hidden_folder\\",
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec![
        "another/",
        "custom_completion.nu",
        "directory_completion/",
        "nushell",
        "test_a/",
        "test_a_symlink",
        "test_b/",
        ".hidden_file",
        ".hidden_folder/",
    ];

    match_suggestions(&expected_paths, &suggestions)
}

#[test]
fn command_watch_with_filecompletion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "watch ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<_> = vec![
        "another\\",
        "custom_completion.nu",
        "directory_completion\\",
        "nushell",
        "test_a\\",
        "test_a_symlink",
        "test_b\\",
        ".hidden_file",
        ".hidden_folder\\",
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec![
        "another/",
        "custom_completion.nu",
        "directory_completion/",
        "nushell",
        "test_a/",
        "test_a_symlink",
        "test_b/",
        ".hidden_file",
        ".hidden_folder/",
    ];

    match_suggestions(&expected_paths, &suggestions)
}

#[rstest]
fn subcommand_completions() {
    let (_, _, mut engine, mut stack) = new_engine();
    let commands = r#"
            $env.config.completions.algorithm = "fuzzy"
            def foo-test-command [] {}
            def "foo-test-command bar" [] {}
            def "foo-test-command aagap bcr" [] {}
            def "food bar" [] {}
        "#;
    assert!(support::merge_input(commands.as_bytes(), &mut engine, &mut stack).is_ok());
    let mut subcommand_completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let prefix = "fod br";
    let suggestions = subcommand_completer.complete(prefix, prefix.len());
    match_suggestions(
        &vec![
            "food bar",
            "foo-test-command bar",
            "foo-test-command aagap bcr",
        ],
        &suggestions,
    );

    let prefix = "foot bar";
    let suggestions = subcommand_completer.complete(prefix, prefix.len());
    match_suggestions(&vec!["foo-test-command bar"], &suggestions);
}

#[test]
fn file_completion_quoted() {
    let (_, _, engine, stack) = new_quote_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "open ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    let test_dir_folder = format!("`{}`", folder("test dir"));
    let expected_paths: Vec<_> = vec![
        "`--help`",
        "`-42`",
        "`-inf`",
        "`4.2`",
        "\'[a] bc.txt\'",
        "`curly-bracket_{.txt`",
        "`semicolon_;.txt`",
        "'square-bracket_[.txt'",
        "`te st.txt`",
        "`te#st.txt`",
        "`te'st.txt`",
        "`te(st).txt`",
        test_dir_folder.as_str(),
    ];

    match_suggestions(&expected_paths, &suggestions);

    let dir: PathBuf = "test dir".into();
    let target_dir = format!("open '{}'", folder(dir.clone()));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    let expected_paths = [
        format!("`{}`", file(dir.join("double quote"))),
        format!("`{}`", file(dir.join("single quote"))),
    ];

    match_suggestions_by_string(&expected_paths, &suggestions)
}

#[test]
fn flag_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    // Test completions for the 'ls' flags
    let suggestions = completer.complete("ls -", 4);

    assert_eq!(18, suggestions.len());

    let expected: Vec<_> = vec![
        "--all",
        "--directory",
        "--du",
        "--full-paths",
        "--help",
        "--long",
        "--mime-type",
        "--short-names",
        "--threads",
        "-a",
        "-D",
        "-d",
        "-f",
        "-h",
        "-l",
        "-m",
        "-s",
        "-t",
    ];

    // Match results
    match_suggestions(&expected, &suggestions);
}

#[test]
fn attribute_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    // Test completions for the 'ls' flags
    let suggestions = completer.complete("@", 1);

    // Only checking for the builtins and not the std attributes
    let expected: Vec<_> = vec!["category", "example", "search-terms"];

    // Match results
    match_suggestions(&expected, &suggestions);
}

#[test]
fn attributable_completions() {
    // Create a new engine
    let (_, _, engine, stack) = new_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    // Test completions for the 'ls' flags
    let suggestions = completer.complete("@example; ", 10);

    let expected: Vec<_> = vec!["def", "export def", "export extern", "extern"];

    // Match results
    match_suggestions(&expected, &suggestions);
}

#[test]
fn folder_with_directorycompletions() {
    // Create a new engine
    let (dir, dir_str, engine, stack) = new_engine();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Test completions for the current folder
    let target_dir = format!("cd {dir_str}{MAIN_SEPARATOR}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        folder(dir.join("another")),
        folder(dir.join("directory_completion")),
        folder(dir.join("test_a")),
        file(dir.join("test_a_symlink")),
        folder(dir.join("test_b")),
        folder(dir.join(".hidden_folder")),
    ];

    #[cfg(windows)]
    {
        let target_dir = format!("cd {dir_str}/");
        let slash_suggestions = completer.complete(&target_dir, target_dir.len());

        let expected_slash_paths: Vec<_> = expected_paths
            .iter()
            .map(|s| s.replace('\\', "/"))
            .collect();

        match_suggestions_by_string(&expected_slash_paths, &slash_suggestions);
    }

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);
}

#[test]
fn folder_with_directorycompletions_with_dots() {
    // Create a new engine
    let (dir, _, engine, stack) = new_engine();
    let dir_str = dir
        .join("directory_completion")
        .join("folder_inside_folder")
        .into_os_string()
        .into_string()
        .unwrap();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Test completions for the current folder
    let target_dir = format!("cd {dir_str}{MAIN_SEPARATOR}..{MAIN_SEPARATOR}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [folder(
        dir.join("directory_completion")
            .join("folder_inside_folder")
            .join("..")
            .join("folder_inside_folder"),
    )];

    #[cfg(windows)]
    {
        let target_dir = format!("cd {dir_str}/../");
        let slash_suggestions = completer.complete(&target_dir, target_dir.len());

        let expected_slash_paths: Vec<_> = expected_paths
            .iter()
            .map(|s| s.replace('\\', "/"))
            .collect();

        match_suggestions_by_string(&expected_slash_paths, &slash_suggestions);
    }

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);
}

#[test]
fn folder_with_directorycompletions_with_three_trailing_dots() {
    // Create a new engine
    let (dir, _, engine, stack) = new_engine();
    let dir_str = dir
        .join("directory_completion")
        .join("folder_inside_folder")
        .into_os_string()
        .into_string()
        .unwrap();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Test completions for the current folder
    let target_dir = format!("cd {dir_str}{MAIN_SEPARATOR}...{MAIN_SEPARATOR}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths = [
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("...")
                .join("another"),
        ),
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("...")
                .join("directory_completion"),
        ),
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("...")
                .join("test_a"),
        ),
        file(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("...")
                .join("test_a_symlink"),
        ),
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("...")
                .join("test_b"),
        ),
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("...")
                .join(".hidden_folder"),
        ),
    ];

    #[cfg(windows)]
    {
        let target_dir = format!("cd {dir_str}/.../");
        let slash_suggestions = completer.complete(&target_dir, target_dir.len());

        let expected_slash_paths: Vec<_> = expected_paths
            .iter()
            .map(|s| s.replace('\\', "/"))
            .collect();

        match_suggestions_by_string(&expected_slash_paths, &slash_suggestions);
    }

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);
}

#[test]
fn folder_with_directorycompletions_do_not_collapse_dots() {
    // Create a new engine
    let (dir, _, engine, stack) = new_engine();
    let dir_str = dir
        .join("directory_completion")
        .join("folder_inside_folder")
        .into_os_string()
        .into_string()
        .unwrap();

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Test completions for the current folder
    let target_dir = format!("cd {dir_str}{MAIN_SEPARATOR}..{MAIN_SEPARATOR}..{MAIN_SEPARATOR}");
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Create the expected values
    let expected_paths: Vec<_> = vec![
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("..")
                .join("..")
                .join("another"),
        ),
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("..")
                .join("..")
                .join("directory_completion"),
        ),
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("..")
                .join("..")
                .join("test_a"),
        ),
        file(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("..")
                .join("..")
                .join("test_a_symlink"),
        ),
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("..")
                .join("..")
                .join("test_b"),
        ),
        folder(
            dir.join("directory_completion")
                .join("folder_inside_folder")
                .join("..")
                .join("..")
                .join(".hidden_folder"),
        ),
    ];

    #[cfg(windows)]
    {
        let target_dir = format!("cd {dir_str}/../../");
        let slash_suggestions = completer.complete(&target_dir, target_dir.len());

        let expected_slash_paths: Vec<_> = expected_paths
            .iter()
            .map(|s| s.replace('\\', "/"))
            .collect();

        match_suggestions_by_string(&expected_slash_paths, &slash_suggestions);
    }

    // Match the results
    match_suggestions_by_string(&expected_paths, &suggestions);
}

#[test]
fn variables_completions() {
    // Create a new engine
    let (_, _, mut engine, mut stack) = new_engine();

    // Add record value as example
    let record = "let actor = { name: 'Tom Hardy', age: 44 }";
    assert!(support::merge_input(record.as_bytes(), &mut engine, &mut stack).is_ok());

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Test completions for $nu
    let suggestions = completer.complete("$nu.", 4);

    assert_eq!(19, suggestions.len());

    let expected: Vec<_> = vec![
        "cache-dir",
        "config-path",
        "current-exe",
        "data-dir",
        "default-config-dir",
        "env-path",
        "history-enabled",
        "history-path",
        "home-path",
        "is-interactive",
        "is-login",
        "loginshell-path",
        "os-info",
        "pid",
        "plugin-path",
        "startup-time",
        "temp-path",
        "user-autoload-dirs",
        "vendor-autoload-dirs",
    ];

    // Match results
    match_suggestions(&expected, &suggestions);

    // Test completions for $nu.h (filter)
    let suggestions = completer.complete("$nu.h", 5);

    assert_eq!(3, suggestions.len());

    let expected: Vec<_> = vec!["history-enabled", "history-path", "home-path"];

    // Match results
    match_suggestions(&expected, &suggestions);

    // Test completions for $nu.os-info
    let suggestions = completer.complete("$nu.os-info.", 12);
    assert_eq!(4, suggestions.len());
    let expected: Vec<_> = vec!["arch", "family", "kernel_version", "name"];
    // Match results
    match_suggestions(&expected, &suggestions);

    // Test completions for custom var
    let suggestions = completer.complete("$actor.", 7);

    assert_eq!(2, suggestions.len());

    let expected: Vec<_> = vec!["age", "name"];

    // Match results
    match_suggestions(&expected, &suggestions);

    // Test completions for custom var (filtering)
    let suggestions = completer.complete("$actor.n", 8);

    assert_eq!(1, suggestions.len());

    let expected: Vec<_> = vec!["name"];

    // Match results
    match_suggestions(&expected, &suggestions);

    // Test completions for $env
    let suggestions = completer.complete("$env.", 5);

    assert_eq!(3, suggestions.len());

    #[cfg(windows)]
    let expected: Vec<_> = vec!["Path", "PWD", "TEST"];
    #[cfg(not(windows))]
    let expected: Vec<_> = vec!["PATH", "PWD", "TEST"];

    // Match results
    match_suggestions(&expected, &suggestions);

    // Test completions for $env
    let suggestions = completer.complete("$env.T", 6);

    assert_eq!(1, suggestions.len());

    let expected: Vec<_> = vec!["TEST"];

    // Match results
    match_suggestions(&expected, &suggestions);

    let suggestions = completer.complete("$", 1);
    let expected: Vec<_> = vec!["$actor", "$env", "$in", "$nu"];

    match_suggestions(&expected, &suggestions);
}

#[test]
fn record_cell_path_completions() {
    let (_, _, mut engine, mut stack) = new_engine();
    let command = r#"let foo = {a: [1 {a: 2}]}; const bar = {a: [1 {a: 2}]}"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let expected: Vec<_> = vec!["a"];
    let completion_str = "$foo.";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&expected, &suggestions);

    let completion_str = "$foo.a.1.";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&expected, &suggestions);

    let completion_str = "$bar.";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&expected, &suggestions);

    let completion_str = "$bar.a.1.";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&expected, &suggestions);

    let completion_str = "{a: [1 {a: 2}]}.a.1.";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&expected, &suggestions);
}

#[test]
fn table_cell_path_completions() {
    let (_, _, mut engine, mut stack) = new_engine();
    let command = r#"let foo = [{a:{b:1}}, {a:{b:2}}]; const bar = [[a b]; [1 2]]"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());
    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let expected: Vec<_> = vec!["a"];
    let completion_str = "$foo.";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&expected, &suggestions);

    let expected: Vec<_> = vec!["b"];
    let completion_str = "$foo.a.";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&expected, &suggestions);

    let expected: Vec<_> = vec!["a", "b"];
    let completion_str = "$bar.";
    let suggestions = completer.complete(completion_str, completion_str.len());
    match_suggestions(&expected, &suggestions);
}

#[test]
fn alias_of_command_and_flags() {
    let (_, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls -l"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let suggestions = completer.complete("ll t", 4);
    #[cfg(windows)]
    let expected_paths: Vec<_> = vec!["test_a\\", "test_a_symlink", "test_b\\"];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec!["test_a/", "test_a_symlink", "test_b/"];

    match_suggestions(&expected_paths, &suggestions)
}

#[test]
fn alias_of_basic_command() {
    let (_, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls "#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let suggestions = completer.complete("ll t", 4);
    #[cfg(windows)]
    let expected_paths: Vec<_> = vec!["test_a\\", "test_a_symlink", "test_b\\"];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec!["test_a/", "test_a_symlink", "test_b/"];

    match_suggestions(&expected_paths, &suggestions)
}

#[test]
fn alias_of_another_alias() {
    let (_, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls -la"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack).is_ok());
    // Create the second alias
    let alias = r#"alias lf = ll -f"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let suggestions = completer.complete("lf t", 4);
    #[cfg(windows)]
    let expected_paths: Vec<_> = vec!["test_a\\", "test_a_symlink", "test_b\\"];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec!["test_a/", "test_a_symlink", "test_b/"];

    match_suggestions(&expected_paths, &suggestions)
}

fn run_external_completion(completer: &str, input: &str) -> Vec<Suggestion> {
    let completer = format!("$env.config.completions.external.completer = {completer}");

    // Create a new engine
    let (_, _, mut engine_state, mut stack) = new_engine();
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
    assert!(engine_state.merge_env(&mut stack).is_ok());

    // Instantiate a new completer
    let mut completer = NuCompleter::new(Arc::new(engine_state), Arc::new(stack));

    completer.complete(input, input.len())
}

#[test]
fn unknown_command_completion() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = "thiscommanddoesnotexist ";
    let suggestions = completer.complete(target_dir, target_dir.len());

    #[cfg(windows)]
    let expected_paths: Vec<_> = vec![
        "another\\",
        "custom_completion.nu",
        "directory_completion\\",
        "nushell",
        "test_a\\",
        "test_a_symlink",
        "test_b\\",
        ".hidden_file",
        ".hidden_folder\\",
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec![
        "another/",
        "custom_completion.nu",
        "directory_completion/",
        "nushell",
        "test_a/",
        "test_a_symlink",
        "test_b/",
        ".hidden_file",
        ".hidden_folder/",
    ];

    match_suggestions(&expected_paths, &suggestions)
}

#[rstest]
fn flagcompletion_triggers_after_cursor(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst -h", 5);
    let expected: Vec<_> = vec!["--help", "--mod", "-h", "-s"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn customcompletion_triggers_after_cursor(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-command c", 11);
    let expected: Vec<_> = vec!["cat", "dog", "eel"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn customcompletion_triggers_after_cursor_piped(mut completer_strings: NuCompleter) {
    let suggestions = completer_strings.complete("my-command c | ls", 11);
    let expected: Vec<_> = vec!["cat", "dog", "eel"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn flagcompletion_triggers_after_cursor_piped(mut completer: NuCompleter) {
    let suggestions = completer.complete("tst -h | ls", 5);
    let expected: Vec<_> = vec!["--help", "--mod", "-h", "-s"];
    match_suggestions(&expected, &suggestions);
}

#[test]
fn filecompletions_triggers_after_cursor() {
    let (_, _, engine, stack) = new_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let suggestions = completer.complete("cp   test_c", 3);

    #[cfg(windows)]
    let expected_paths: Vec<_> = vec![
        "another\\",
        "custom_completion.nu",
        "directory_completion\\",
        "nushell",
        "test_a\\",
        "test_a_symlink",
        "test_b\\",
        ".hidden_file",
        ".hidden_folder\\",
    ];
    #[cfg(not(windows))]
    let expected_paths: Vec<_> = vec![
        "another/",
        "custom_completion.nu",
        "directory_completion/",
        "nushell",
        "test_a/",
        "test_a_symlink",
        "test_b/",
        ".hidden_file",
        ".hidden_folder/",
    ];

    match_suggestions(&expected_paths, &suggestions);
}

#[rstest]
fn extern_custom_completion_positional(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam ", 5);
    let expected: Vec<_> = vec!["cat", "dog", "eel"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn extern_custom_completion_long_flag_1(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam --foo=", 11);
    let expected: Vec<_> = vec!["cat", "dog", "eel"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn extern_custom_completion_long_flag_2(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam --foo ", 11);
    let expected: Vec<_> = vec!["cat", "dog", "eel"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn extern_custom_completion_long_flag_short(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam -f ", 8);
    let expected: Vec<_> = vec!["cat", "dog", "eel"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn extern_custom_completion_short_flag(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam -b ", 8);
    let expected: Vec<_> = vec!["cat", "dog", "eel"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn extern_complete_flags(mut extern_completer: NuCompleter) {
    let suggestions = extern_completer.complete("spam -", 6);
    let expected: Vec<_> = vec!["--foo", "-b", "-f"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn custom_completer_triggers_cursor_before_word(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("cmd foo  bar", 8);
    let expected: Vec<_> = vec!["cmd", "foo", ""];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn custom_completer_triggers_cursor_on_word_left_boundary(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("cmd foo bar", 8);
    let expected: Vec<_> = vec!["cmd", "foo", ""];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn custom_completer_triggers_cursor_next_to_word(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("cmd foo bar", 11);
    let expected: Vec<_> = vec!["cmd", "foo", "bar"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn custom_completer_triggers_cursor_after_word(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("cmd foo bar ", 12);
    let expected: Vec<_> = vec!["cmd", "foo", "bar", ""];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn sort_fuzzy_completions_in_alphabetical_order(mut fuzzy_alpha_sort_completer: NuCompleter) {
    let suggestions = fuzzy_alpha_sort_completer.complete("ls nu", 5);
    // Even though "nushell" is a better match, it should come second because
    // the completions should be sorted in alphabetical order
    match_suggestions(&vec!["custom_completion.nu", "nushell"], &suggestions);
}

#[test]
fn exact_match() {
    let (dir, _, engine, stack) = new_partial_engine();

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    let target_dir = format!("open {}", folder(dir.join("pArTiAL")));
    let suggestions = completer.complete(&target_dir, target_dir.len());

    // Since it's an exact match, only 'partial' should be suggested, not
    // 'partial-a' and stuff. Implemented in #13302
    match_suggestions(
        &vec![file(dir.join("partial").join("hello.txt")).as_str()],
        &suggestions,
    );
}

#[ignore = "was reverted, still needs fixing"]
#[rstest]
fn alias_offset_bug_7648() {
    let (_, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ea = ^$env.EDITOR /tmp/test.s"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

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
    let (_, _, mut engine, mut stack) = new_engine();

    // Create an alias
    let alias = r#"alias ll = ls -l"#;
    assert!(support::merge_input(alias.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));

    // Issue #7754
    // Nushell crashes when an alias name is shorter than the alias command
    // and the alias command contains pipes.
    // This crashes before PR #7756
    let _suggestions = completer.complete("ll -a | c", 9);
}

#[rstest]
fn nested_block(mut completer: NuCompleter) {
    let expected: Vec<_> = vec!["--help", "--mod", "-h", "-s"];

    let suggestions = completer.complete("somecmd | lines | each { tst - }", 30);
    match_suggestions(&expected, &suggestions);

    let suggestions = completer.complete("somecmd | lines | each { tst -}", 30);
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn incomplete_nested_block(mut completer: NuCompleter) {
    let suggestions = completer.complete("somecmd | lines | each { tst -", 30);
    let expected: Vec<_> = vec!["--help", "--mod", "-h", "-s"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn deeply_nested_block(mut completer: NuCompleter) {
    let suggestions = completer.complete(
        "somecmd | lines | each { print ([each (print) (tst -)]) }",
        52,
    );
    let expected: Vec<_> = vec!["--help", "--mod", "-h", "-s"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn operator_completions(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("1 ", 2);
    // == != > < >= <= in not-in
    // + - * / // mod **
    // 5 bit-xxx
    assert_eq!(20, suggestions.len());
    let suggestions = custom_completer.complete("1 bit-s", 7);
    let expected: Vec<_> = vec!["bit-shl", "bit-shr"];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("'str' ", 6);
    // == != > < >= <= in not-in
    // has not-has starts-with ends-with
    // =~ !~ like not-like ++
    assert_eq!(17, suggestions.len());
    let suggestions = custom_completer.complete("'str' +", 7);
    let expected: Vec<_> = vec!["++"];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("1ms ", 4);
    // == != > < >= <= in not-in
    // + - * / // mod
    assert_eq!(14, suggestions.len());
    let suggestions = custom_completer.complete("1ms /", 5);
    let expected: Vec<_> = vec!["/", "//"];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("..2 ", 4);
    // == != in not-in has not-has
    assert_eq!(6, suggestions.len());
    let suggestions = custom_completer.complete("..2 h", 5);
    let expected: Vec<_> = vec!["has"];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("[[];[]] ", 8);
    // == != in not-in has not-has ++
    assert_eq!(7, suggestions.len());
    let suggestions = custom_completer.complete("[[];[]] h", 9);
    let expected: Vec<_> = vec!["has"];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("(date now) ", 11);
    // == != > < >= <= in not-in
    assert_eq!(8, suggestions.len());
    let suggestions = custom_completer.complete("(date now) <", 12);
    let expected: Vec<_> = vec!["<", "<="];
    match_suggestions(&expected, &suggestions);

    // default operators for all types
    let expected: Vec<_> = vec!["!=", "==", "in", "not-in"];
    let suggestions = custom_completer.complete("{1} ", 4);
    match_suggestions(&expected, &suggestions);
    let suggestions = custom_completer.complete("null ", 5);
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn cell_path_operator_completions(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("[1].0 ", 6);
    // == != > < >= <= in not-in
    // + - * / // mod **
    // 5 bit-xxx
    assert_eq!(20, suggestions.len());
    let suggestions = custom_completer.complete("[1].0 bit-s", 11);
    let expected: Vec<_> = vec!["bit-shl", "bit-shr"];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("{'foo': [1, 1kb]}.foo.1 ", 24);
    // == != > < >= <= in not-in
    // + - * / // mod
    assert_eq!(14, suggestions.len());
    let suggestions = custom_completer.complete("{'foo': [1, 1kb]}.foo.1 mo", 26);
    let expected: Vec<_> = vec!["mod"];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("const f = {'foo': [1, '1']}; $f.foo.1 ", 38);
    // == != > < >= <= in not-in
    // has not-has starts-with ends-with
    // =~ !~ like not-like ++
    assert_eq!(17, suggestions.len());
    let suggestions = custom_completer.complete("const f = {'foo': [1, '1']}; $f.foo.1 ++", 40);
    let expected: Vec<_> = vec!["++"];
    match_suggestions(&expected, &suggestions);
}

#[rstest]
fn assignment_operator_completions(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("mut foo = ''; $foo ", 19);
    // == != > < >= <= in not-in
    // has not-has starts-with ends-with
    // =~ !~ like not-like ++
    // = ++=
    assert_eq!(19, suggestions.len());
    let suggestions = custom_completer.complete("mut foo = ''; $foo ++", 21);
    let expected: Vec<_> = vec!["++", "++="];
    match_suggestions(&expected, &suggestions);

    // == != > < >= <= in not-in
    // =
    let suggestions = custom_completer.complete("mut foo = date now; $foo ", 25);
    assert_eq!(9, suggestions.len());
    let suggestions = custom_completer.complete("mut foo = date now; $foo =", 26);
    let expected: Vec<_> = vec!["=", "=="];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("mut foo = date now; $foo ", 25);
    // == != > < >= <= in not-in
    // =
    assert_eq!(9, suggestions.len());
    let suggestions = custom_completer.complete("mut foo = date now; $foo =", 26);
    let expected: Vec<_> = vec!["=", "=="];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("mut foo = 1ms; $foo ", 20);
    // == != > < >= <= in not-in
    // + - * / // mod
    // = += -= *= /=
    assert_eq!(19, suggestions.len());
    let suggestions = custom_completer.complete("mut foo = 1ms; $foo +", 21);
    let expected: Vec<_> = vec!["+", "+="];
    match_suggestions(&expected, &suggestions);

    // default operators for all mutables
    let expected: Vec<_> = vec!["!=", "=", "==", "in", "not-in"];
    let suggestions = custom_completer.complete("mut foo = null; $foo ", 21);
    match_suggestions(&expected, &suggestions);

    // $env should be considered mutable
    let suggestions = custom_completer.complete("$env.config.keybindings ", 24);
    // == != in not-in
    // has not-has ++=
    // = ++=
    assert_eq!(9, suggestions.len());
    let expected: Vec<_> = vec!["++", "++="];
    let suggestions = custom_completer.complete("$env.config.keybindings +", 25);
    match_suggestions(&expected, &suggestions);

    // all operators for type any
    let suggestions = custom_completer.complete("ls | where name ", 16);
    assert_eq!(30, suggestions.len());
    let expected: Vec<_> = vec!["starts-with"];
    let suggestions = custom_completer.complete("ls | where name starts", 22);
    match_suggestions(&expected, &suggestions);
}

#[test]
fn cellpath_assignment_operator_completions() {
    let (_, _, mut engine, mut stack) = new_engine();
    let command = r#"mut foo = {'foo': [1, '1']}"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    let completion_str = "$foo.foo.1 ";
    let suggestions = completer.complete(completion_str, completion_str.len());
    // == != > < >= <= in not-in
    // has not-has starts-with ends-with
    // =~ !~ like not-like ++
    // = ++=
    assert_eq!(19, suggestions.len());
    let completion_str = "$foo.foo.1 ++";
    let suggestions = completer.complete(completion_str, completion_str.len());
    let expected: Vec<_> = vec!["++", "++="];
    match_suggestions(&expected, &suggestions);

    let (_, _, mut engine, mut stack) = new_engine();
    let command = r#"mut foo = {'foo': [1, (date now)]}"#;
    assert!(support::merge_input(command.as_bytes(), &mut engine, &mut stack).is_ok());

    let mut completer = NuCompleter::new(Arc::new(engine), Arc::new(stack));
    let completion_str = "$foo.foo.1 ";
    let suggestions = completer.complete(completion_str, completion_str.len());
    // == != > < >= <= in not-in
    // =
    assert_eq!(9, suggestions.len());
    let completion_str = "$foo.foo.1 =";
    let suggestions = completer.complete(completion_str, completion_str.len());
    let expected: Vec<_> = vec!["=", "=="];
    match_suggestions(&expected, &suggestions);
}

// TODO: type inference
#[ignore]
#[rstest]
fn type_inferenced_operator_completions(mut custom_completer: NuCompleter) {
    let suggestions = custom_completer.complete("let f = {'foo': [1, '1']}; $f.foo.1 ", 36);
    // == != > < >= <= in not-in
    // has not-has starts-with ends-with
    // =~ !~ like not-like ++
    assert_eq!(17, suggestions.len());
    let suggestions = custom_completer.complete("const f = {'foo': [1, '1']}; $f.foo.1 ++", 38);
    let expected: Vec<_> = vec!["++"];
    match_suggestions(&expected, &suggestions);

    let suggestions = custom_completer.complete("mut foo = [(date now)]; $foo.0 ", 31);
    // == != > < >= <= in not-in
    // =
    assert_eq!(9, suggestions.len());
    let suggestions = custom_completer.complete("mut foo = [(date now)]; $foo.0 =", 32);
    let expected: Vec<_> = vec!["=", "=="];
    match_suggestions(&expected, &suggestions);
}
