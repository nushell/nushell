use miette::{Diagnostic, LabeledSpan};
use nu_cmd_lang::{Alias, Def};
use nu_parser::parse;
use nu_protocol::engine::{EngineState, StateWorkingSet};

use nu_cmd_lang::AttrDeprecated;

#[test]
pub fn test_deprecated_attribute() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    working_set.add_decl(Box::new(Def));
    working_set.add_decl(Box::new(Alias));
    working_set.add_decl(Box::new(AttrDeprecated));

    // test deprecation with no message
    let source = br#"
    @deprecated
    def foo [] {}
    "#;
    let _ = parse(&mut working_set, None, source, false);

    // there should be no warning until the command is called
    assert!(working_set.parse_errors.is_empty());
    assert!(working_set.parse_warnings.is_empty());

    let source = b"foo";
    let _ = parse(&mut working_set, None, source, false);

    // command called, there should be a deprecation warning
    assert!(working_set.parse_errors.is_empty());
    assert!(!working_set.parse_warnings.is_empty());
    let labels: Vec<LabeledSpan> = working_set.parse_warnings[0].labels().unwrap().collect();
    let label = labels.first().unwrap().label().unwrap();
    assert!(label.contains("foo is deprecated"));
    working_set.parse_warnings.clear();

    // test deprecation with message
    let source = br#"
    @deprecated "Use new-command instead"
    def old-command [] {}

    old-command
    "#;
    let _ = parse(&mut working_set, None, source, false);

    assert!(working_set.parse_errors.is_empty());
    assert!(!working_set.parse_warnings.is_empty());

    let help = &working_set.parse_warnings[0].help().unwrap().to_string();
    assert!(help.contains("Use new-command instead"));
}

#[test]
pub fn test_deprecated_attribute_flag() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    working_set.add_decl(Box::new(Def));
    working_set.add_decl(Box::new(Alias));
    working_set.add_decl(Box::new(AttrDeprecated));

    let source = br#"
    @deprecated "Use foo instead of bar" --flag bar
    @deprecated "Use foo instead of baz" --flag baz
    def old-command [--foo, --bar, --baz] {}
    old-command --foo
    old-command --bar
    old-command --baz
    old-command --foo --bar --baz
    "#;
    let _ = parse(&mut working_set, None, source, false);

    assert!(working_set.parse_errors.is_empty());
    assert!(!working_set.parse_warnings.is_empty());

    let help = &working_set.parse_warnings[0].help().unwrap().to_string();
    assert!(help.contains("Use foo instead of bar"));

    let help = &working_set.parse_warnings[1].help().unwrap().to_string();
    assert!(help.contains("Use foo instead of baz"));

    let help = &working_set.parse_warnings[2].help().unwrap().to_string();
    assert!(help.contains("Use foo instead of bar"));

    let help = &working_set.parse_warnings[3].help().unwrap().to_string();
    assert!(help.contains("Use foo instead of baz"));
}

#[test]
pub fn test_deprecated_attribute_since_remove() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    working_set.add_decl(Box::new(Def));
    working_set.add_decl(Box::new(Alias));
    working_set.add_decl(Box::new(AttrDeprecated));

    let source = br#"
    @deprecated --since 0.10000.0 --remove 1.0
    def old-command [] {}
    old-command
    "#;
    let _ = parse(&mut working_set, None, source, false);

    assert!(working_set.parse_errors.is_empty());
    assert!(!working_set.parse_warnings.is_empty());

    let labels: Vec<LabeledSpan> = working_set.parse_warnings[0].labels().unwrap().collect();
    let label = labels.first().unwrap().label().unwrap();
    assert!(label.contains("0.10000.0"));
    assert!(label.contains("1.0"));
}
