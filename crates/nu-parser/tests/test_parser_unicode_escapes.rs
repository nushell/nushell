#![cfg(test)]

//use nu_parser::ParseError;
use nu_parser::*;
use nu_protocol::{
    //ast::{Expr, Expression, PipelineElement},
    ast::{Expr, PipelineElement},
    //engine::{Command, EngineState, Stack, StateWorkingSet},
    engine::{EngineState, StateWorkingSet},
    //Signature, SyntaxShape,
};

pub fn do_test (test: &[u8], expected:&str, error_contains:Option<&str>) {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(
        &mut working_set,
        None,
        test,
        true,
        &[],
    );


    match err {
        None => {
            assert_eq!(block.len(), 1);
            let expressions = &block[0];
            assert_eq!(expressions.len(), 1);
            if let PipelineElement::Expression(_, expr) = &expressions[0] {
                assert_eq!(expr.expr, Expr::String(expected.to_string()))
            } else {
                panic!("Not an expression")
            }
        }
        Some(pev) => {
            match error_contains {
                None => {
                    panic!("Err:{:#?}", pev);
                }
                Some(contains_string) => {
                    let full_err = format!("{:#?}", pev);
                    assert!(full_err.contains(contains_string), "Expected error containing {}, instead got {}", contains_string, full_err);
                }
            }

        }
    }
    
    
}

// cases that all should work
#[test]
pub fn parse_unicode_many() {
    pub struct Tc(&'static [u8], &'static str);

    let test_vec = vec![
        Tc(  b"\"hello \\u{6e}\\u{000075}\\u{073}hell\"", "hello nushell"),
        // template: Tc(br#""<string literal without #'s>"", "<Rust literal comparand>")
        Tc(br#""\u006enu\u0075\u0073\u0073""#, "nnuuss"),
        Tc(br#""hello \u{6e}\u{000075}\u{073}hell""#, "hello nushell"),
        Tc(br#""abc""#, "abc"),
        Tc(br#""\u{39}8\u{10ffff}""#,"98\u{10ffff}"),  // shouldn't work?
        //Tc(br#""#,""),
        //Tc(br#""#,""),
        //Tc(br#""#,""),
        
    ];

    for tci in test_vec {
        println!("Expecting: {}", tci.1);
        do_test( tci.0, tci.1, None);
    }    
}

// cases that all should fail (in expected way)
#[test]
pub fn parse_unicode_many_fail() {
    // input, substring of expected failure
    pub struct Tc(&'static [u8], &'static str);

    let test_vec = vec![
        // template: Tc(br#""<string literal without #'s>"", "<pattern in expected error>")
        Tc(br#""\u06e""#, "any shape"),   // 4digit too short, next char is EOF
        Tc(br#""\u06ex""#, "any shape"),  // 4digit too short, next char is non-hex-digit
        Tc(br#""hello \u{6e""#, "any shape"),   // extended, missing close delim
        Tc(br#""\u{39}8\u{000000000000000000000000000000000000000000000037}""#,"any shape"),  // shouldn't work?
        Tc(br#""\u{110000}""#,"any shape"),  // max unicode <= 0x10ffff?
        //Tc(br#""#,""),
        //Tc(br#""#,""),
        //Tc(br#""#,""),
        
    ];

    for tci in test_vec {
        println!("Expecting failure containing: {}", tci.1);
        do_test( tci.0, "--success not expected--", Some(tci.1));
    }    
}
