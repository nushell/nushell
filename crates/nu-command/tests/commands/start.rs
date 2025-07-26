use super::*;
use nu_engine::test_help::{convert_single_value_to_cmd_args, eval_block_with_input};
use nu_engine::{current_dir, eval_expression};
use nu_protocol::{
    PipelineData, Span, Spanned, Type, Value,
    ast::Call,
    engine::{EngineState, Stack, StateWorkingSet},
};
use std::path::PathBuf;

/// Create a minimal test engine state and stack to run commands against.
fn create_test_context() -> (EngineState, Stack) {
    let mut engine_state = EngineState::new();
    let mut stack = Stack::new();

    // A working set is needed for storing definitions in the engine state.
    let _working_set = StateWorkingSet::new(&mut engine_state);

    // Add the `Start` command to the engine state so we can run it.
    let start_cmd = Start;
    engine_state.add_cmd(Box::new(start_cmd));

    (engine_state, stack)
}

#[test]
fn test_start_valid_url() {
    let (engine_state, mut stack) = create_test_context();

    // For safety in tests, we won't actually open anything,
    // but we can still check that the command resolves as a URL
    // and attempts to run. Typically, you'd mock `open::commands` if needed.

    // Create call for: `start https://www.example.com`
    let path = "https://www.example.com".to_string();
    let span = Span::test_data();
    let call = Call::test(
        "start",
        // The arguments for `start` are just the path in this case
        vec![Value::string(path, span)],
    );

    let result = Start.run(&engine_state, &mut stack, &call, PipelineData::empty);

    assert!(
        result.is_ok(),
        "Expected successful run with a valid URL, got error: {:?}",
        result.err()
    );
}

#[test]
fn test_start_valid_local_path() {
    let (engine_state, mut stack) = create_test_context();

    // Here we'll simulate opening the current directory (`.`).
    let path = ".".to_string();
    let span = Span::test_data();
    let call = Call::test("start", vec![Value::string(path, span)]);

    let result = Start.run(&engine_state, &mut stack, &call, PipelineData::empty);

    // If the environment is correctly set, it should succeed.
    // If you're running in a CI environment or restricted environment
    // this might fail, so you may need to mock `open` calls.
    assert!(
        result.is_ok(),
        "Expected successful run opening current directory, got error: {:?}",
        result.err()
    );
}

#[test]
fn test_start_nonexistent_local_path() {
    let (engine_state, mut stack) = create_test_context();

    // Create an obviously invalid path
    let path = "this_file_does_not_exist_hopefully.txt".to_string();
    let span = Span::test_data();
    let call = Call::test("start", vec![Value::string(path, span)]);

    let result = Start.run(&engine_state, &mut stack, &call, PipelineData::empty);

    // We expect an error since the file does not exist
    assert!(
        result.is_err(),
        "Expected an error for a non-existent file path"
    );

    if let Err(ShellError::GenericError { error, .. }) = result {
        assert!(
            error.contains("Cannot find file or URL"),
            "Expected 'Cannot find file or URL' in error, found: {}",
            error
        );
    } else {
        panic!("Unexpected error type, expected ShellError::GenericError");
    }
}

