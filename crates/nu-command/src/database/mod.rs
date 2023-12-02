mod commands;
mod values;

use commands::add_commands_decls;
use nu_protocol::engine::StateWorkingSet;
pub use values::{
    convert_sqlite_row_to_nu_value, convert_sqlite_value_to_nu_value, open_connection_in_memory,
    SQLiteDatabase,
};

pub fn add_database_decls(working_set: &mut StateWorkingSet) {
    add_commands_decls(working_set);
}
