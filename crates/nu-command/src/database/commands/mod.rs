mod command;
mod open;
mod query;

use command::Database;
use nu_protocol::engine::StateWorkingSet;
use open::OpenDb;
use query::QueryDb;

pub fn add_database_decls(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    // Series commands
    bind_command!(Database, QueryDb, OpenDb);
}
