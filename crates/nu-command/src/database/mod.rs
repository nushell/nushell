mod commands;
mod values;

mod expressions;
pub use values::{
    convert_sqlite_row_to_nu_value, convert_sqlite_value_to_nu_value, open_and_read_sqlite_db,
    open_connection_in_memory, read_sqlite_db, SQLiteDatabase,
};
use nu_protocol::engine::StateWorkingSet;
pub use commands::add_commands_decls;
pub use expressions::add_expression_decls;

pub fn add_database_decls(working_set: &mut StateWorkingSet) {
    add_commands_decls(working_set);
    add_expression_decls(working_set);
}
