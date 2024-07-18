use nu_engine::eval_block;
use nu_parser::parse;
use nu_path::{AbsolutePathBuf, PathBuf};
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, ShellError, Span, Value,
};
use nu_test_support::fs;
use reedline::Suggestion;
use std::path::MAIN_SEPARATOR;

fn create_default_context() -> EngineState {
    nu_command::add_shell_command_context(nu_cmd_lang::create_default_context())
}

// creates a new engine with the current path into the completions fixtures folder
pub fn new_engine() -> (AbsolutePathBuf, String, EngineState, Stack) {
    // Target folder inside assets
    let dir = fs::fixtures().join("completions");
    let dir_str = dir
        .clone()
        .into_os_string()
        .into_string()
        .unwrap_or_default();

    // Create a new engine with default context
    let mut engine_state = create_default_context();

    // Add $nu
    engine_state.generate_nu_constant();

    // New stack
    let mut stack = Stack::new();

    // Add pwd as env var
    stack.add_env_var(
        "PWD".to_string(),
        Value::string(dir_str.clone(), nu_protocol::Span::new(0, dir_str.len())),
    );
    stack.add_env_var(
        "TEST".to_string(),
        Value::string(
            "NUSHELL".to_string(),
            nu_protocol::Span::new(0, dir_str.len()),
        ),
    );
    #[cfg(windows)]
    stack.add_env_var(
        "Path".to_string(),
        Value::string(
            "c:\\some\\path;c:\\some\\other\\path".to_string(),
            nu_protocol::Span::new(0, dir_str.len()),
        ),
    );
    #[cfg(not(windows))]
    stack.add_env_var(
        "PATH".to_string(),
        Value::string(
            "/some/path:/some/other/path".to_string(),
            nu_protocol::Span::new(0, dir_str.len()),
        ),
    );

    // Merge environment into the permanent state
    let merge_result = engine_state.merge_env(&mut stack, &dir);
    assert!(merge_result.is_ok());

    (dir, dir_str, engine_state, stack)
}

// creates a new engine with the current path into the completions fixtures folder
pub fn new_dotnu_engine() -> (AbsolutePathBuf, String, EngineState, Stack) {
    // Target folder inside assets
    let dir = fs::fixtures().join("dotnu_completions");
    let dir_str = dir
        .clone()
        .into_os_string()
        .into_string()
        .unwrap_or_default();
    let dir_span = nu_protocol::Span::new(0, dir_str.len());

    // Create a new engine with default context
    let mut engine_state = create_default_context();

    // Add $nu
    engine_state.generate_nu_constant();

    // New stack
    let mut stack = Stack::new();

    // Add pwd as env var
    stack.add_env_var("PWD".to_string(), Value::string(dir_str.clone(), dir_span));
    stack.add_env_var(
        "TEST".to_string(),
        Value::string("NUSHELL".to_string(), dir_span),
    );

    stack.add_env_var(
        "NU_LIB_DIRS".to_string(),
        Value::List {
            vals: vec![
                Value::string(file(dir.join("lib-dir1")), dir_span),
                Value::string(file(dir.join("lib-dir2")), dir_span),
                Value::string(file(dir.join("lib-dir3")), dir_span),
            ],
            internal_span: dir_span,
        },
    );

    // Merge environment into the permanent state
    let merge_result = engine_state.merge_env(&mut stack, &dir);
    assert!(merge_result.is_ok());

    (dir, dir_str, engine_state, stack)
}

pub fn new_quote_engine() -> (AbsolutePathBuf, String, EngineState, Stack) {
    // Target folder inside assets
    let dir = fs::fixtures().join("quoted_completions");
    let dir_str = dir
        .clone()
        .into_os_string()
        .into_string()
        .unwrap_or_default();

    // Create a new engine with default context
    let mut engine_state = create_default_context();

    // New stack
    let mut stack = Stack::new();

    // Add pwd as env var
    stack.add_env_var(
        "PWD".to_string(),
        Value::string(dir_str.clone(), nu_protocol::Span::new(0, dir_str.len())),
    );
    stack.add_env_var(
        "TEST".to_string(),
        Value::string(
            "NUSHELL".to_string(),
            nu_protocol::Span::new(0, dir_str.len()),
        ),
    );

    // Merge environment into the permanent state
    let merge_result = engine_state.merge_env(&mut stack, &dir);
    assert!(merge_result.is_ok());

    (dir, dir_str, engine_state, stack)
}

pub fn new_partial_engine() -> (AbsolutePathBuf, String, EngineState, Stack) {
    // Target folder inside assets
    let dir = fs::fixtures().join("partial_completions");
    let dir_str = dir
        .clone()
        .into_os_string()
        .into_string()
        .unwrap_or_default();

    // Create a new engine with default context
    let mut engine_state = create_default_context();

    // New stack
    let mut stack = Stack::new();

    // Add pwd as env var
    stack.add_env_var(
        "PWD".to_string(),
        Value::string(dir_str.clone(), nu_protocol::Span::new(0, dir_str.len())),
    );
    stack.add_env_var(
        "TEST".to_string(),
        Value::string(
            "NUSHELL".to_string(),
            nu_protocol::Span::new(0, dir_str.len()),
        ),
    );

    // Merge environment into the permanent state
    let merge_result = engine_state.merge_env(&mut stack, &dir);
    assert!(merge_result.is_ok());

    (dir, dir_str, engine_state, stack)
}

// match a list of suggestions with the expected values
pub fn match_suggestions(expected: Vec<String>, suggestions: Vec<Suggestion>) {
    let expected_len = expected.len();
    let suggestions_len = suggestions.len();
    if expected_len != suggestions_len {
        panic!(
            "\nexpected {expected_len} suggestions but got {suggestions_len}: \n\
            Suggestions: {suggestions:#?} \n\
            Expected: {expected:#?}\n"
        )
    }
    assert_eq!(
        expected,
        suggestions
            .into_iter()
            .map(|it| it.value)
            .collect::<Vec<_>>()
    );
}

// append the separator to the converted path
pub fn folder(path: impl Into<PathBuf>) -> String {
    let mut converted_path = file(path);
    converted_path.push(MAIN_SEPARATOR);
    converted_path
}

// convert a given path to string
pub fn file(path: impl Into<PathBuf>) -> String {
    path.into().into_os_string().into_string().unwrap()
}

// merge_input executes the given input into the engine
// and merges the state
pub fn merge_input(
    input: &[u8],
    engine_state: &mut EngineState,
    stack: &mut Stack,
    dir: AbsolutePathBuf,
) -> Result<(), ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);

        let block = parse(&mut working_set, None, input, false);

        assert!(working_set.parse_errors.is_empty());

        (block, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    assert!(eval_block::<WithoutDebug>(
        engine_state,
        stack,
        &block,
        PipelineData::Value(Value::nothing(Span::unknown()), None),
    )
    .is_ok());

    // Merge environment into the permanent state
    engine_state.merge_env(stack, &dir)
}
