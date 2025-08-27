mod query_db;
mod schema;
mod to_sqlite;

use nu_protocol::engine::StateWorkingSet;
use query_db::QueryDb;
use schema::SchemaDb;
use to_sqlite::ToSqliteDb;

pub fn add_commands_decls(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    // Series commands
    bind_command!(ToSqliteDb, QueryDb, SchemaDb);
}
