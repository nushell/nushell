mod commands;
mod values;

mod expressions;
pub use commands::add_commands_decls;
pub use expressions::add_expression_decls;
use nu_protocol::engine::StateWorkingSet;
pub use values::{
    convert_sqlite_row_to_nu_value, convert_sqlite_value_to_nu_value, open_and_read_sqlite_db,
    open_connection_in_memory, read_sqlite_db, SQLiteDatabase,
};

pub fn add_database_decls(working_set: &mut StateWorkingSet) {
    add_commands_decls(working_set);
    add_expression_decls(working_set);
}
