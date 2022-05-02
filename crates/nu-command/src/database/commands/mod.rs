mod and;
mod collect;
mod command;
mod describe;
mod from;
mod limit;
mod open;
mod or;
mod order_by;
mod query;
mod schema;
mod select;
mod where_;

// Temporal module to create Query objects
mod testing;
use testing::TestingDb;

use nu_protocol::engine::StateWorkingSet;

use and::AndDb;
use collect::CollectDb;
use command::Database;
use describe::DescribeDb;
use from::FromDb;
use limit::LimitDb;
use open::OpenDb;
use or::OrDb;
use order_by::OrderByDb;
use query::QueryDb;
use schema::SchemaDb;
use select::ProjectionDb;
use where_::WhereDb;

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
        AndDb,
        CollectDb,
        Database,
        DescribeDb,
        FromDb,
        QueryDb,
        LimitDb,
        ProjectionDb,
        OpenDb,
        OrderByDb,
        OrDb,
        SchemaDb,
        TestingDb,
        WhereDb
    );
}
