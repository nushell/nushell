mod collect;
mod command;
mod describe;
mod from;
mod open;
mod query;
mod schema;
mod select;

// Temporal module to create Query objects
mod testing;
use testing::TestingDb;

use nu_protocol::engine::StateWorkingSet;

use collect::CollectDb;
use command::Database;
use describe::DescribeDb;
use from::FromDb;
use open::OpenDb;
use query::QueryDb;
use schema::SchemaDb;
use select::ProjectionDb;

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
    bind_command!(
        CollectDb,
        Database,
        DescribeDb,
        FromDb,
        QueryDb,
        ProjectionDb,
        OpenDb,
        SchemaDb,
        TestingDb
    );
}
