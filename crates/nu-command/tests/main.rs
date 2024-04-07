use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Category, PositionalArg, Span,
};
use quickcheck_macros::quickcheck;

mod commands;
mod format_conversions;

fn create_default_context() -> EngineState {
    nu_command::add_shell_command_context(nu_cmd_lang::create_default_context())
}

#[quickcheck]
fn quickcheck_parse(data: String) -> bool {
    let (tokens, err) = nu_parser::lex(data.as_bytes(), 0, b"", b"", true);

    if err.is_none() {
        let context = create_default_context();
        {
            let mut working_set = StateWorkingSet::new(&context);
            let _ = working_set.add_file("quickcheck".into(), data.as_bytes());

            let _ =
                nu_parser::parse_block(&mut working_set, &tokens, Span::new(0, 0), false, false);
        }
    }
    true
}

#[test]
fn arguments_end_period() {
    fn ends_period(cmd_name: &str, ty: &str, arg: PositionalArg, failures: &mut Vec<String>) {
        let arg_name = arg.name;
        let desc = arg.desc;
        if !desc.ends_with('.') {
            failures.push(format!(
                "{cmd_name} {ty} argument \"{arg_name}\": \"{desc}\""
            ));
        }
    }

    let ctx = crate::create_default_context();
    let decls = ctx.get_decls_sorted(true);
    let mut failures = Vec::new();

    for (name_bytes, decl_id) in decls {
        let cmd = ctx.get_decl(decl_id);
        let cmd_name = String::from_utf8_lossy(&name_bytes);
        let signature = cmd.signature();

        for arg in signature.required_positional {
            ends_period(&cmd_name, "required", arg, &mut failures);
        }

        for arg in signature.optional_positional {
            ends_period(&cmd_name, "optional", arg, &mut failures);
        }

        if let Some(arg) = signature.rest_positional {
            ends_period(&cmd_name, "rest", arg, &mut failures);
        }
    }

    assert!(
        failures.is_empty(),
        "Command argument description does not end with a period:\n{}",
        failures.join("\n")
    );
}

#[test]
fn arguments_start_uppercase() {
    fn starts_uppercase(cmd_name: &str, ty: &str, arg: PositionalArg, failures: &mut Vec<String>) {
        let arg_name = arg.name;
        let desc = arg.desc;

        // Check lowercase to allow usage to contain syntax like:
        //
        // "`as` keyword …"
        if desc.starts_with(|u: char| u.is_lowercase()) {
            failures.push(format!(
                "{cmd_name} {ty} argument \"{arg_name}\": \"{desc}\""
            ));
        }
    }

    let ctx = crate::create_default_context();
    let decls = ctx.get_decls_sorted(true);
    let mut failures = Vec::new();

    for (name_bytes, decl_id) in decls {
        let cmd = ctx.get_decl(decl_id);
        let cmd_name = String::from_utf8_lossy(&name_bytes);
        let signature = cmd.signature();

        for arg in signature.required_positional {
            starts_uppercase(&cmd_name, "required", arg, &mut failures);
        }

        for arg in signature.optional_positional {
            starts_uppercase(&cmd_name, "optional", arg, &mut failures);
        }

        if let Some(arg) = signature.rest_positional {
            starts_uppercase(&cmd_name, "rest", arg, &mut failures);
        }
    }

    assert!(
        failures.is_empty(),
        "Command argument description does not end with a period:\n{}",
        failures.join("\n")
    );
}

#[test]
fn signature_name_matches_command_name() {
    let ctx = create_default_context();
    let decls = ctx.get_decls_sorted(true);
    let mut failures = Vec::new();

    for (name_bytes, decl_id) in decls {
        let cmd = ctx.get_decl(decl_id);
        let cmd_name = String::from_utf8_lossy(&name_bytes);
        let sig_name = cmd.signature().name;
        let category = cmd.signature().category;

        if cmd_name != sig_name {
            failures.push(format!(
                "{cmd_name} ({category:?}): Signature name \"{sig_name}\" is not equal to the command name \"{cmd_name}\""
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Name mismatch:\n{}",
        failures.join("\n")
    );
}

#[test]
fn commands_declare_input_output_types() {
    let ctx = create_default_context();
    let decls = ctx.get_decls_sorted(true);
    let mut failures = Vec::new();

    for (_, decl_id) in decls {
        let cmd = ctx.get_decl(decl_id);
        let sig_name = cmd.signature().name;
        let category = cmd.signature().category;

        if matches!(category, Category::Removed | Category::Custom(_)) {
            // Deprecated/Removed commands don't have to conform
            // TODO: also upgrade the `--features dataframe` commands
            continue;
        }

        if cmd.signature().input_output_types.is_empty() {
            failures.push(format!(
                "{sig_name} ({category:?}): No pipeline input/output type signatures found"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Command missing type annotations:\n{}",
        failures.join("\n")
    );
}

#[test]
fn no_search_term_duplicates() {
    let ctx = crate::create_default_context();
    let decls = ctx.get_decls_sorted(true);
    let mut failures = Vec::new();

    for (name_bytes, decl_id) in decls {
        let cmd = ctx.get_decl(decl_id);
        let cmd_name = String::from_utf8_lossy(&name_bytes);
        let search_terms = cmd.search_terms();
        let category = cmd.signature().category;

        for search_term in search_terms {
            if cmd_name.contains(search_term) {
                failures.push(format!("{cmd_name} ({category:?}): Search term \"{search_term}\" is substring of command name \"{cmd_name}\""));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Duplication in search terms:\n{}",
        failures.join("\n")
    );
}

#[test]
fn usage_end_period() {
    let ctx = crate::create_default_context();
    let decls = ctx.get_decls_sorted(true);
    let mut failures = Vec::new();

    for (name_bytes, decl_id) in decls {
        let cmd = ctx.get_decl(decl_id);
        let cmd_name = String::from_utf8_lossy(&name_bytes);
        let usage = cmd.usage();

        if !usage.ends_with('.') {
            failures.push(format!("{cmd_name}: \"{usage}\""));
        }
    }

    assert!(
        failures.is_empty(),
        "Command usage does not end with a period:\n{}",
        failures.join("\n")
    );
}

#[test]
fn usage_start_uppercase() {
    let ctx = crate::create_default_context();
    let decls = ctx.get_decls_sorted(true);
    let mut failures = Vec::new();

    for (name_bytes, decl_id) in decls {
        let cmd = ctx.get_decl(decl_id);
        let cmd_name = String::from_utf8_lossy(&name_bytes);
        let usage = cmd.usage();

        // Check lowercase to allow usage to contain syntax like:
        //
        // "`$env.FOO = ...`"
        if usage.starts_with(|u: char| u.is_lowercase()) {
            failures.push(format!("{cmd_name}: \"{usage}\""));
        }
    }

    assert!(
        failures.is_empty(),
        "Command usage does not start with an uppercase letter:\n{}",
        failures.join("\n")
    );
}
