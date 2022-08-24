use nu_command::create_default_context;
use nu_protocol::engine::StateWorkingSet;
use quickcheck_macros::quickcheck;

mod commands;
mod format_conversions;

// use nu_engine::EvaluationContext;

#[quickcheck]
fn quickcheck_parse(data: String) -> bool {
    let (tokens, err) = nu_parser::lex(data.as_bytes(), 0, b"", b"", true);
    let (lite_block, err2) = nu_parser::lite_parse(&tokens);

    if err.is_none() && err2.is_none() {
        let context = create_default_context();
        {
            let mut working_set = StateWorkingSet::new(&context);
            working_set.add_file("quickcheck".into(), data.as_bytes());

            let _ = nu_parser::parse_block(&mut working_set, &lite_block, false, &[], false);
        }
    }
    true
}

#[test]
fn signature_name_matches_command_name() {
    let ctx = crate::create_default_context();
    let decls = ctx.get_decl_ids_sorted(true);
    let mut failures = Vec::new();

    for decl_id in decls {
        let cmd = ctx.get_decl(decl_id);
        let cmd_name = cmd.name();
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
fn no_search_term_duplicates() {
    let ctx = crate::create_default_context();
    let decls = ctx.get_decl_ids_sorted(true);
    let mut failures = Vec::new();

    for decl_id in decls {
        let cmd = ctx.get_decl(decl_id);
        let cmd_name = cmd.name();
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
