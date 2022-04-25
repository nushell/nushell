mod commands;
mod values;
mod expressions;

use nu_protocol::engine::StateWorkingSet;

pub use commands::add_commands_decls;
pub use expressions::add_expression_decls;
pub(crate) use values::SQLiteDatabase;

pub fn add_database_decls(working_set: &mut StateWorkingSet) {
    add_commands_decls(working_set);
    add_expression_decls(working_set);
}
