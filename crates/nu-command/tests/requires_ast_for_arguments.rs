//! Regression guard for `Command::requires_ast_for_arguments`.
//!
//! When true, the IR compiler retains full argument AST nodes (`IrAstRef`) for that
//! decl — expensive and an obstacle to IR-only calls. After migration phases 0–1, no
//! builtin should opt in.
//!
//! This test walks all decls and asserts the set of opt-ins equals
//! [`ALLOWED_REQUIRES_AST_FOR_ARGUMENTS`]. That list is **empty on purpose**: it locks
//! in “zero opt-ins.” If someone adds `requires_ast_for_arguments => true` without
//! updating the allowlist, CI fails. Prefer redesigning the command instead of
//! re-adding an entry; see `devdocs/ir_call_migration.md`.

use nu_protocol::engine::EngineState;
use std::collections::BTreeSet;

/// Expected names of decls that may still set `requires_ast_for_arguments` to true.
///
/// Empty after phase 1 — not dead code. An empty allowlist means we expect *none* and
/// will fail if any appear. Only add a name for a deliberate, temporary exception
/// (and document it in `devdocs/ir_call_migration.md`).
const ALLOWED_REQUIRES_AST_FOR_ARGUMENTS: &[&str] = &[];

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
