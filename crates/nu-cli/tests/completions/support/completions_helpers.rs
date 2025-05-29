use nu_engine::eval_block;
use nu_parser::parse;
use nu_path::{AbsolutePathBuf, PathBuf};
use nu_protocol::{
    PipelineData, ShellError, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
};
use nu_test_support::fs;
use reedline::Suggestion;
use std::path::MAIN_SEPARATOR;

fn create_default_context() -> EngineState {
    nu_command::add_shell_command_context(nu_cmd_lang::create_default_context())
}

pub fn new_engine_helper(pwd: AbsolutePathBuf) -> (AbsolutePathBuf, String, EngineState, Stack) {
    let pwd_str = pwd
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
        Value::string(pwd_str.clone(), nu_protocol::Span::new(0, pwd_str.len())),
    );
    stack.add_env_var(
        "TEST".to_string(),
        Value::string(
            "NUSHELL".to_string(),
            nu_protocol::Span::new(0, pwd_str.len()),
        ),
    );
    #[cfg(windows)]
    stack.add_env_var(
        "Path".to_string(),
        Value::string(
            "c:\\some\\path;c:\\some\\other\\path".to_string(),
            nu_protocol::Span::new(0, pwd_str.len()),
        ),
    );
    #[cfg(not(windows))]
    stack.add_env_var(
        "PATH".to_string(),
        Value::string(
            "/some/path:/some/other/path".to_string(),
            nu_protocol::Span::new(0, pwd_str.len()),
        ),
    );

    // Merge environment into the permanent state
    let merge_result = engine_state.merge_env(&mut stack);
    assert!(merge_result.is_ok());

    (pwd, pwd_str, engine_state, stack)
}

/// creates a new engine with the current path in the completions fixtures folder
pub fn new_engine() -> (AbsolutePathBuf, String, EngineState, Stack) {
    new_engine_helper(fs::fixtures().join("completions"))
}

/// Adds pseudo PATH env for external completion tests
pub fn new_external_engine() -> EngineState {
    let mut engine = create_default_context();
    let dir = fs::fixtures().join("external_completions").join("path");
    let dir_str = dir.to_string_lossy().to_string();
    let internal_span = nu_protocol::Span::new(0, dir_str.len());
    engine.add_env_var(
        "PATH".to_string(),
        Value::List {
            vals: vec![Value::String {
                val: dir_str,
                internal_span,
            }],
            internal_span,
        },
    );
    engine
}

/// creates a new engine with the current path in the dotnu_completions fixtures folder
pub fn new_dotnu_engine() -> (AbsolutePathBuf, String, EngineState, Stack) {
    // Target folder inside assets
    let dir = fs::fixtures().join("dotnu_completions");
    let (dir, dir_str, mut engine_state, mut stack) = new_engine_helper(dir);
    let dir_span = nu_protocol::Span::new(0, dir_str.len());

    // const $NU_LIB_DIRS
    let mut working_set = StateWorkingSet::new(&engine_state);
    let var_id = working_set.add_variable(
        b"$NU_LIB_DIRS".into(),
        Span::unknown(),
        nu_protocol::Type::List(Box::new(nu_protocol::Type::String)),
        false,
    );
    working_set.set_variable_const_val(
        var_id,
        Value::test_list(vec![
            Value::string(file(dir.join("lib-dir1")), dir_span),
            Value::string(file(dir.join("lib-dir3")), dir_span),
        ]),
    );
    let _ = engine_state.merge_delta(working_set.render());

    stack.add_env_var(
        "NU_LIB_DIRS".into(),
        Value::test_list(vec![
            Value::string(file(dir.join("lib-dir2")), dir_span),
            Value::string(file(dir.join("lib-dir3")), dir_span),
        ]),
    );

    // Merge environment into the permanent state
    let merge_result = engine_state.merge_env(&mut stack);
    assert!(merge_result.is_ok());

    (dir, dir_str, engine_state, stack)
}

pub fn new_quote_engine() -> (AbsolutePathBuf, String, EngineState, Stack) {
    new_engine_helper(fs::fixtures().join("quoted_completions"))
}

pub fn new_partial_engine() -> (AbsolutePathBuf, String, EngineState, Stack) {
    new_engine_helper(fs::fixtures().join("partial_completions"))
}

/// match a list of suggestions with the expected values
pub fn match_suggestions(expected: &Vec<&str>, suggestions: &Vec<Suggestion>) {
    let expected_len = expected.len();
    let suggestions_len = suggestions.len();
    if expected_len != suggestions_len {
        panic!(
            "\nexpected {expected_len} suggestions but got {suggestions_len}: \n\
            Suggestions: {suggestions:#?} \n\
            Expected: {expected:#?}\n"
        )
    }

    let suggestions_str = suggestions
        .iter()
        .map(|it| it.value.as_str())
        .collect::<Vec<_>>();

    assert_eq!(expected, &suggestions_str);
}

/// match a list of suggestions with the expected values
pub fn match_suggestions_by_string(expected: &[String], suggestions: &Vec<Suggestion>) {
    let expected = expected.iter().map(|it| it.as_str()).collect::<Vec<_>>();
    match_suggestions(&expected, suggestions);
}

/// append the separator to the converted path
pub fn folder(path: impl Into<PathBuf>) -> String {
    let mut converted_path = file(path);
    converted_path.push(MAIN_SEPARATOR);
    converted_path
}

/// convert a given path to string
pub fn file(path: impl Into<PathBuf>) -> String {
    path.into().into_os_string().into_string().unwrap()
}

/// merge_input executes the given input into the engine
/// and merges the state
pub fn merge_input(
    input: &[u8],
    engine_state: &mut EngineState,
    stack: &mut Stack,
) -> Result<(), ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);

        let block = parse(&mut working_set, None, input, false);

        assert!(working_set.parse_errors.is_empty());

        (block, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    assert!(
        eval_block::<WithoutDebug>(
            engine_state,
            stack,
            &block,
            PipelineData::Value(Value::nothing(Span::unknown()), None),
        )
        .is_ok()
    );

    // Merge environment into the permanent state
    engine_state.merge_env(stack)
}
