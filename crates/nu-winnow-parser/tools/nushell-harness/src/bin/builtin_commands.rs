//! Print every command a fresh `nu` resolves at the top level, one per line
//! as `name<TAB>type`, sorted by name. `type` is nu-protocol's
//! `CommandType` (`Builtin`, `Keyword`, `Custom` for the standard library's
//! prelude).
//!
//! ```text
//! cargo run --release --bin builtin-commands
//! ```
//!
//! `tools/scripts/gen-builtin-commands.nu` turns the output into
//! `src/builtin_commands.rs`, so the parser knows exactly the commands this
//! harness's nu-parser knows.

use nushell_harness::engine;

pub fn main() {
    let engine_state = engine(true);
    for (name, decl_id) in engine_state.get_decls_sorted(false) {
        let command_type = engine_state.get_decl(decl_id).command_type();
        println!("{}\t{command_type:?}", String::from_utf8_lossy(&name));
    }
}
