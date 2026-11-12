use crate::repl::tests::{TestResult, run_test};
use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
// avoid feeding strings containing parens to regex.  Does not end well.
#[case(": arga help")]
#[case("argb help")]
#[case("optional, default: 20")]
#[case(": f1 switch")]
#[case(": f2 named no default")]
#[case(": f3 named default 3")]
#[case("default: 33")]
#[case("--help: Display the help message")]
fn can_get_help(#[case] exp_result: &str) -> TestResult {
    run_test(
        &format!(
            "def t [a:string, # arga help
            b:int=20, # argb help
            --f1, # f1 switch help
            --f2:string, # f2 named no default
            --f3:int=33 # f3 named default 3
            ] {{ true }};
            help t | ansi strip | find `{exp_result}` | get 0 | str replace --all --regex '^(.*({exp_result}).*)$' '$2'",
        ),
        exp_result,
    )
}

// Code in backticks that isn't highlighted as Nushell is italic and underlined in the theme's
// `shape_external` color, never the faint style that is unreadable on some dark terminals. Only
// the color is taken, so `cyan_bold` and a bold record draw no bold, and a record without `fg`
// leaves the terminal's default color, as flag names do. `color` is the color's SGR parameter
// with its leading `;`, empty for the default color. `name` and
// `git status --short` parse as an external command and its arguments, and a bare `alias`
// doesn't parse at all. They all get that one color, not the theme's separate colors for
// resolved externals (yellow) and external arguments (green). `str trim` keeps its theme colors.
#[rstest]
#[case::color_name("'red'", ";31")]
#[case::color_with_bold("'cyan_bold'", ";36")]
#[case::style_record("{fg: purple, attr: b}", ";35")]
#[case::style_record_without_fg("{attr: b}", "")]
fn help_code_fallback_uses_shape_external(
    #[case] shape_external: &str,
    #[case] color: &str,
) -> Result {
    let code = format!(
        "
        $env.config.use_ansi_coloring = true
        $env.config.color_config.shape_external = {shape_external}
        $env.config.color_config.shape_external_resolved = 'yellow'
        $env.config.color_config.shape_externalarg = 'green'
        $env.config.color_config.shape_internalcall = 'blue'
        # Select just the `name` column, unlike `alias`. Run `str trim` or `git status --short`.
        def foo [] {{}}
        help foo
        "
    );
    let outcome: String = test().run(code)?;
    let description = outcome
        .lines()
        .find(|line| line.starts_with("Select just"))
        .expect("help shows the description");
    assert_contains("\x1b[3;39m\x1b[34mstr trim\x1b[0m", description);
    assert_contains(format!("\x1b[3;39m\x1b[4{color}mname\x1b[0m"), description);
    assert_contains(format!("\x1b[3;4{color}malias\x1b[0m"), description);
    for word in ["git", "status", "--short"] {
        assert_contains(format!("\x1b[4{color}m{word}\x1b[0m"), description);
    }
    assert_contains_not("\x1b[33m", description);
    assert_contains_not("\x1b[32m", description);
    assert_contains_not("\x1b[2;", &outcome);
    assert_contains_not("\x1b[2m", &outcome);
    Ok(())
}

// nu-highlight treats the end of a snippet as the cursor. A snippet in backticks that ends in a
// bracket keeps its theme color, without the matching-bracket underline, which would read as the
// fallback style.
#[test]
fn help_code_has_no_matching_bracket_underline() -> Result {
    let code = "
        $env.config.use_ansi_coloring = true
        $env.config.color_config.shape_closure = 'blue'
        $env.config.color_config.shape_block = 'blue'
        # Run `do {}` or `(1 + 2)`.
        def foo [] {}
        help foo
    ";
    let outcome: String = test().run(code)?;
    let description = outcome
        .lines()
        .find(|line| line.starts_with("Run"))
        .expect("help shows the description");
    for bracket in ["{", "}", "(", ")"] {
        assert_contains(format!("\x1b[34m{bracket}\x1b[0m"), description);
    }
    Ok(())
}
