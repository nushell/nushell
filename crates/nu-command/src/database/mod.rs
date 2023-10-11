mod commands;

use commands::add_commands_decls;
use nu_protocol::engine::StateWorkingSet;

pub fn add_database_decls(working_set: &mut StateWorkingSet) {
    add_commands_decls(working_set);
}
