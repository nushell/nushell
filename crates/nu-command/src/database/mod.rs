mod commands;
mod values;

use commands::add_commands_decls;

pub use values::{
    convert_sqlite_row_to_nu_value, convert_sqlite_value_to_nu_value, open_connection_in_memory,
    open_connection_in_memory_custom, values_to_sql, SQLiteDatabase, MEMORY_DB,
};

use nu_protocol::engine::StateWorkingSet;

pub fn add_database_decls(working_set: &mut StateWorkingSet) {
    add_commands_decls(working_set);
}
