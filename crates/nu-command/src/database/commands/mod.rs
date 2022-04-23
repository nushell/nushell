mod collect;
mod command;
mod from;
mod open;
mod query;
mod select;
mod utils;

use collect::CollectDb;
use command::Database;
use from::FromDb;
use nu_protocol::engine::StateWorkingSet;
use open::OpenDb;
use query::QueryDb;
use select::SelectDb;

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
    bind_command!(CollectDb, Database, FromDb, QueryDb, SelectDb, OpenDb);
}
