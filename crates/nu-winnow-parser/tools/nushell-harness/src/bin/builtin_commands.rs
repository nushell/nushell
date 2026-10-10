//! Print every command a fresh `nu` resolves at the top level, one per line
//! as `name<TAB>type<TAB>row-condition`, sorted by name. `type` is
//! nu-protocol's `CommandType` (`Builtin`, `Keyword`, `Custom` for the
//! standard library's prelude); `row-condition` is `true` when the command's
//! first positional parameter is a row condition (`any`, `take while`).
//!
//! ```text
//! cargo run --release --bin builtin-commands
//! ```
//!
//! `tools/scripts/gen-builtin-commands.nu` turns the output into
//! `src/builtin_commands.rs`, so the parser knows exactly the commands this
//! harness's nu-parser knows.

use nu_protocol::SyntaxShape;
use nushell_harness::engine;

pub fn main() {
    let engine_state = engine(true);
    for (name, decl_id) in engine_state.get_decls_sorted(false) {
        let decl = engine_state.get_decl(decl_id);
        let command_type = decl.command_type();
        let row_condition = decl
            .signature()
            .required_positional
            .first()
            .is_some_and(|positional| positional.shape == SyntaxShape::RowCondition);
        println!("{}\t{command_type:?}\t{row_condition}", String::from_utf8_lossy(&name));
    }
}
