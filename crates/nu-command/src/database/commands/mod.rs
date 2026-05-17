mod into_sqlite;
mod query;
mod query_db;
mod schema;

use into_sqlite::IntoSqliteDb;
use nu_protocol::engine::StateWorkingSet;
use query::Query;
use query_db::QueryDb;
use schema::SchemaDb;

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
    bind_command!(IntoSqliteDb, Query, QueryDb, SchemaDb);
}
