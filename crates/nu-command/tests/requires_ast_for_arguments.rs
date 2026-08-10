//! Inventory guard for commands that still opt into IR argument AST retention.
//!
//! New entries must be deliberate — see `devdocs/ir_call_migration.md`.
//! Prefer redesigning the command so it does not need `requires_ast_for_arguments`.

use nu_protocol::engine::EngineState;
use std::collections::BTreeSet;

/// Commands that are still allowed to require AST argument nodes under IR evaluation.
///
/// Update this list only when adding or removing a deliberate opt-in.
const ALLOWED_REQUIRES_AST_FOR_ARGUMENTS: &[&str] =
    &["attr example", "default", "export-env", "metadata"];

fn engine_state_with_commands() -> EngineState {
    let engine_state = nu_cmd_lang::create_default_context();
    nu_command::add_shell_command_context(engine_state)
}

#[test]
fn requires_ast_for_arguments_inventory() {
    let engine_state = engine_state_with_commands();

    let mut actual: BTreeSet<String> = BTreeSet::new();
    for (name_bytes, decl_id) in engine_state.get_decls_sorted(true) {
        let decl = engine_state.get_decl(decl_id);
        if decl.requires_ast_for_arguments() {
            let name = String::from_utf8_lossy(&name_bytes).into_owned();
            actual.insert(name);
        }
    }

    let expected: BTreeSet<String> = ALLOWED_REQUIRES_AST_FOR_ARGUMENTS
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    assert_eq!(
        actual,
        expected,
        "requires_ast_for_arguments inventory changed.\n\
         unexpected (new opt-ins or renames): {:?}\n\
         missing (removed without updating allowlist): {:?}\n\
         See devdocs/ir_call_migration.md",
        actual.difference(&expected).collect::<Vec<_>>(),
        expected.difference(&actual).collect::<Vec<_>>(),
    );
}
