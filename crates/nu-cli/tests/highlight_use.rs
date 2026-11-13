use nu_parser::parse;
use nu_protocol::engine::StateWorkingSet;
use nu_std::load_standard_library;

/// The highlighter sets `skip_module_load` so typing `use std` does not
/// parse-time-load the standard library. Execution still loads it.
#[test]
fn skip_module_load_does_not_parse_std() {
    let mut engine = nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
    assert!(load_standard_library(&mut engine).is_ok());
    let modules_before = engine.num_modules();

    let mut skip = StateWorkingSet::new(&engine);
    skip.skip_module_load = true;
    parse(&mut skip, None, b"use std/iter", false);
    assert!(
        skip.parse_errors.is_empty(),
        "highlight parse should not report ModuleNotFound: {:?}",
        skip.parse_errors
    );
    assert_eq!(
        skip.num_modules(),
        modules_before,
        "skip_module_load must not add modules"
    );

    let mut load = StateWorkingSet::new(&engine);
    parse(&mut load, None, b"use std/iter", false);
    assert!(
        load.parse_errors.is_empty(),
        "execute parse of `use std/iter` should succeed: {:?}",
        load.parse_errors
    );
    assert!(
        load.num_modules() > modules_before,
        "execute parse of `use std/iter` must load the module"
    );
}
