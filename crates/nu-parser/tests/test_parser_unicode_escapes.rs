#![cfg(test)]

use nu_parser::*;
use nu_protocol::{
    ast::Expr,
    engine::{EngineState, StateWorkingSet},
};

pub fn do_test(test: &[u8], expected: &str, error_contains: Option<&str>) {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let block = parse(&mut working_set, None, test, true);

    match working_set.parse_errors.first() {
        None => {
            assert_eq!(block.len(), 1);
            let pipeline = &block.pipelines[0];
            assert_eq!(pipeline.len(), 1);
            let element = &pipeline.elements[0];
            assert!(element.redirection.is_none());
            assert_eq!(element.expr.expr, Expr::String(expected.to_string()));
        }
        Some(pev) => match error_contains {
            None => {
                panic!("Err:{pev:#?}");
            }
            Some(contains_string) => {
                let full_err = format!("{pev:#?}");
                assert!(
                    full_err.contains(contains_string),
                    "Expected error containing {contains_string}, instead got {full_err}"
                );
            }
        },
    }
}

// cases that all should work
#[test]
pub fn unicode_escapes_in_strings() {
    pub struct Tc(&'static [u8], &'static str);

    let test_vec = vec![
        Tc(b"\"hello \\u{6e}\\u{000075}\\u{073}hell\"", "hello nushell"),
        // template: Tc(br#""<string literal without #'s>"", "<Rust literal comparand>")
        //deprecated Tc(br#""\u006enu\u0075\u0073\u0073""#, "nnuuss"),
        Tc(br#""hello \u{6e}\u{000075}\u{073}hell""#, "hello nushell"),
        Tc(br#""\u{39}8\u{10ffff}""#, "98\u{10ffff}"),
        Tc(br#""abc\u{41}""#, "abcA"), // at end of string
        Tc(br#""\u{41}abc""#, "Aabc"), // at start of string
        Tc(br#""\u{a}""#, "\n"),       // single digit
    ];

    for tci in test_vec {
        println!("Expecting: {}", tci.1);
        do_test(tci.0, tci.1, None);
    }
}

// cases that all should fail (in expected way)
#[test]
pub fn unicode_escapes_in_strings_expected_failures() {
    // input, substring of expected failure
    pub struct Tc(&'static [u8], &'static str);

    let test_vec = vec![
        // template: Tc(br#""<string literal without #'s>"", "<pattern in expected error>")
        //deprecated Tc(br#""\u06e""#, "any shape"), // 4digit too short, next char is EOF
        //deprecatedTc(br#""\u06ex""#, "any shape"), // 4digit too short, next char is non-hex-digit
        Tc(br#""hello \u{6e""#, "missing '}'"), // extended, missing close delim
        Tc(
            br#""\u{39}8\u{000000000000000000000000000000000000000000000037}""#,
            "must be 1-6 hex digits",
        ), // hex too long, but small value
        Tc(br#""\u{110000}""#, "max value 10FFF"), // max unicode <= 0x10ffff
    ];

    for tci in test_vec {
        println!("Expecting failure containing: {}", tci.1);
        do_test(tci.0, "--success not expected--", Some(tci.1));
    }
}
