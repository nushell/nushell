// Conversions between value and sqlparser objects
pub mod conversions;

mod alias;
mod and;
mod collect;
mod describe;
mod from;
mod group_by;
mod join;
mod limit;
mod open;
mod or;
mod order_by;
mod query;
mod schema;
mod select;
mod to_db;
mod where_;

// Temporal module to create Query objects
mod testing;
use testing::TestingDb;

use nu_protocol::engine::StateWorkingSet;

use alias::AliasDb;
use and::AndDb;
use collect::CollectDb;
pub(crate) use describe::DescribeDb;
pub(crate) use from::FromDb;
use group_by::GroupByDb;
use join::JoinDb;
use limit::LimitDb;
use open::OpenDb;
use or::OrDb;
use order_by::OrderByDb;
use query::QueryDb;
use schema::SchemaDb;
pub(crate) use select::ProjectionDb;
pub(crate) use to_db::ToDataBase;
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
        ToDataBase,
        AliasDb,
        AndDb,
        CollectDb,
        DescribeDb,
        FromDb,
        GroupByDb,
        JoinDb,
        LimitDb,
        OpenDb,
        OrderByDb,
        OrDb,
        QueryDb,
        ProjectionDb,
        SchemaDb,
        TestingDb,
        WhereDb
    );
}
