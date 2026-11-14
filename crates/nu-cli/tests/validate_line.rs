use nu_cli::NuValidator;
use nu_parser::parse;
use nu_protocol::engine::{EngineState, StateWorkingSet};
use nu_test_support::{fs::Stub::FileWithContent, playground::Playground};
use reedline::{ValidationResult, Validator};
use rstest::rstest;
use std::sync::Arc;

fn engine() -> EngineState {
    nu_command::add_shell_command_context(nu_cmd_lang::create_default_context())
}

fn is_incomplete(engine: EngineState, line: &str) -> bool {
    let validator = NuValidator {
        engine_state: Arc::new(engine),
    };
    matches!(validator.validate(line), ValidationResult::Incomplete)
}

/// Typing more can finish these, so the REPL keeps reading.
#[rstest]
#[case::block("if true {")]
#[case::list("[1 2")]
#[case::subexpression("(1 + ")]
#[case::string("\"abc")]
#[case::parameters("def foo [")]
#[case::second_line("[1\n2")]
#[case::only_a_bracket("[")]
#[case::only_a_brace("{")]
#[case::only_a_paren("(")]
#[case::only_a_quote("\"")]
#[case::trailing_space("[1 2 ")]
#[case::record("{a: 1")]
#[case::interpolation("$\"abc(")]
fn unfinished_line_is_incomplete(#[case] line: &str) {
    assert!(is_incomplete(engine(), line));
}

/// A one-byte file has its error at offset 0, which is the offset right after the line.
#[rstest]
#[case::bracket("validate_one_bracket", "[")]
#[case::brace("validate_one_brace", "{")]
#[case::paren("validate_one_paren", "(")]
#[case::double_quote("validate_one_double_quote", "\"")]
#[case::single_quote("validate_one_single_quote", "'")]
#[case::backtick("validate_one_backtick", "`")]
fn one_byte_loaded_file_is_not_incomplete(#[case] topic: &str, #[case] contents: &str) {
    Playground::setup(topic, |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("one.nu", contents)]);
        let path = dirs.test().join("one.nu");

        let line = format!("use `{}`", path.display());
        assert!(!is_incomplete(engine(), &line));
    });
}

/// An unclosed delimiter in a file the line loads is an error to show, or the REPL traps you at
/// the continuation prompt with nothing you could type to get out.
#[rstest]
#[case::use_file("validate_use", "use")]
#[case::overlay_use("validate_overlay", "overlay use")]
#[case::module("validate_module", "module")]
#[case::source("validate_source", "source")]
fn unclosed_delimiter_in_a_loaded_file_is_not_incomplete(
    #[case] topic: &str,
    #[case] command: &str,
) {
    Playground::setup(topic, |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("broken.nu", "export def foo [ {")]);
        let path = dirs.test().join("broken.nu");

        let line = format!("{command} `{}`", path.display());
        assert!(!is_incomplete(engine(), &line));
    });
}

/// The same goes for `use <name>`, which reads the module's file again when it changed.
#[test]
fn unclosed_delimiter_in_a_changed_module_is_not_incomplete() {
    Playground::setup("validate_use_by_name", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("spam.nu", "export def foo [] { 'foo' }")]);
        let path = dirs.test().join("spam.nu");

        let mut engine = engine();
        let mut working_set = StateWorkingSet::new(&engine);
        let line = format!("use `{}`", path.display());
        parse(&mut working_set, None, line.as_bytes(), false);
        assert!(working_set.parse_errors.is_empty());
        let delta = working_set.render();
        engine.merge_delta(delta).expect("merging the first `use`");

        std::fs::write(&path, "export def foo [ {").expect("editing the module");
        assert!(!is_incomplete(engine, "use spam"));
    });
}

/// A line that loads a broken file can still be unfinished itself.
#[test]
fn unclosed_delimiter_in_the_line_wins_over_one_in_a_file() {
    Playground::setup("validate_both", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("broken.nu", "export def foo [ {")]);
        let path = dirs.test().join("broken.nu");

        let line = format!("use `{}`; if true {{", path.display());
        assert!(is_incomplete(engine(), &line));
    });
}
