//! End-to-end completion behavior and diagnostics.

use assert_cmd::cargo_bin;
use nu_test_support::prelude::*;
use rstest::rstest;
use std::process::Command;

fn completing_with(attribute: &str, body: &str) -> String {
    format!(
        "{attribute}def comp [token: record] {{ {body} }}\n\
         def use-it [arg: string@comp] {{ $arg }}\n\
         'use-it a' | commandline complete"
    )
}

/// Completer examples should type their token record.
#[test]
fn documented_completer_examples_declare_a_typed_token() -> Result {
    let examples: Vec<String> = test().run(
        "scope commands
         | each {|command| $command.examples | each {|example| $example.example } }
         | flatten
         | where {|example| $example =~ 'token' }",
    )?;

    assert!(
        !examples.is_empty(),
        "no completer examples found at all — has the query gone stale?"
    );

    let untyped: Vec<&String> = examples
        .iter()
        .filter(|example| !example.contains("token: record"))
        .collect();

    assert!(
        untyped.is_empty(),
        "these completer examples still declare an untyped `token`: {untyped:#?}"
    );
    Ok(())
}

/// Everything `source` wrote to stderr, run with no logging beyond `extra`.
fn completion_stderr(source: &str, extra: &[&str]) -> Result<String> {
    let output = Command::new(cargo_bin!())
        .args(["--no-config-file", "--no-std-lib"])
        .args(extra)
        .args(["-c", source])
        .output()?;

    assert!(
        output.status.success(),
        "expected the completion to be survivable, but nu failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8_lossy(&output.stderr).into_owned())
}

fn completion_log(source: &str) -> Result<String> {
    completion_stderr(source, &["--log-level", "error"])
}

/// Malformed completer results are logged without failing the shell.
#[rstest]
#[case::block_failed(
    "error make { msg: 'completer exploded' }",
    &["failed to eval completer block", "completer exploded"]
)]
#[case::completions_field_unusable(
    "{completions: 'nope'}",
    &["a completer's `completions` is not usable", "Can't convert to list"]
)]
#[case::suggestion_span_unusable(
    "[{value: a, span: 'nope'}]",
    &["a completer's suggestion span is not usable", "Can't convert to record"]
)]
#[case::unknown_envelope_field(
    "{completions: [a], bogus: 1}",
    &["a completion envelope has no `bogus` field", "completions, options, fallback"]
)]
fn a_completers_mistake_is_explained_in_the_log(
    #[case] body: &str,
    #[case] expected: &[&str],
) -> Result {
    let log = completion_log(&format!("{} | ignore", completing_with("", body)))?;

    for fragment in expected {
        assert!(
            log.contains(fragment),
            "the log never mentioned {fragment:?}\nfull log: {log}"
        );
    }
    Ok(())
}

/// Unknown field still offered.
#[test]
fn a_suggestion_with_an_unknown_field_is_still_offered() -> Result {
    let source = completing_with("", "[{value: alpha, display: 'alpha (a)'}]");

    assert_eq!(test().run::<Vec<String>>(&source)?, ["alpha"]);
    Ok(())
}

/// Bare value is one suggestion.
#[test]
fn a_bare_value_is_read_as_one_suggestion() -> Result {
    let source = completing_with("", "'alpha'");

    assert_eq!(test().run::<Vec<String>>(&source)?, ["alpha"]);
    Ok(())
}

/// Legacy positional bridge warns once.
#[test]
fn the_legacy_positional_bridge_warns_once_without_a_log_level() -> Result {
    let complete = "'use-it a' | commandline complete | ignore";
    let stderr = completion_stderr(
        &format!(
            "def comp [context: string, pos: int] {{ [alpha] }}\n\
             def use-it [arg: string@comp] {{ $arg }}\n\
             {complete}\n{complete}"
        ),
        &[],
    )?;

    assert!(
        stderr.contains("Positional completer input deprecated"),
        "the bridge warned silently: {stderr}"
    );
    assert!(
        stderr.contains("`context`, `pos`"),
        "the warning never named the parameters to migrate: {stderr}"
    );
    assert_eq!(
        stderr
            .matches("Positional completer input deprecated")
            .count(),
        1,
        "a completer must not warn again on every use: {stderr}"
    );
    Ok(())
}

const NEEDS_A_TERMINAL: &str = "nu::shell::interactive_completer_needs_a_terminal";

/// Interactive completers require a terminal; ordinary ones remain available.
#[rstest]
#[case::carries_the_attribute("@interactive\n", true)]
#[case::plain_completer("", false)]
fn only_an_interactive_completer_is_refused_without_a_terminal(
    #[case] attribute: &str,
    #[case] refused: bool,
) -> Result {
    let source = completing_with(attribute, "[alpha]");

    match test().run::<Vec<String>>(&source) {
        Ok(completions) => {
            assert!(
                !refused,
                "expected a refusal, but it ran and returned {completions:?}"
            );
            assert_eq!(
                completions,
                ["alpha"],
                "a plain completer must still answer"
            );
        }
        Err(err) => {
            assert!(refused, "a plain completer must not be refused: {err:?}");
            match err.shell()? {
                ShellError::Generic(generic) => assert_eq!(generic.code, NEEDS_A_TERMINAL),
                other => panic!("expected the refusal, got {other:?}"),
            }
        }
    }
    Ok(())
}

/// Input inspection still works for interactive completers.
#[rstest]
#[case::input_record("--input | get token.text", "a")]
#[case::input_buffer("--input | get buffer", "use-it a")]
fn a_refused_completer_can_still_be_inspected(
    #[case] tail: &str,
    #[case] expected: &str,
) -> Result {
    let source = format!(
        "@interactive\n\
         def comp [token: record] {{ [alpha] }}\n\
         def use-it [arg: string@comp] {{ $arg }}\n\
         'use-it a' | commandline complete {tail}"
    );

    let got: String = test().run(&source)?;
    assert_eq!(got, expected);
    Ok(())
}

/// Built-in completion sources bypass custom completers.
#[test]
fn a_builtin_source_is_unaffected_by_the_refusal() -> Result {
    let source = format!(
        "{} --type command",
        completing_with("@interactive\n", "[alpha]")
    );

    let refused = test().run::<Vec<String>>(&source).is_err();
    assert!(
        !refused,
        "--type runs no completer, so it must not be refused"
    );
    Ok(())
}
