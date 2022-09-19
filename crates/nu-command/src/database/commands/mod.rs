// Conversions between value and sqlparser objects
pub mod conversions;

mod and;
mod as_;
mod collect;
mod describe;
mod from_table;
mod group_by;
mod into_db;
mod into_sqlite;
mod join;
mod limit;
mod open_db;
mod or;
mod order_by;
mod query_db;
mod schema;
mod select;
mod where_;

// Temporal module to create Query objects
mod testing_db;
use testing_db::TestingDb;

use and::AndDb;
use as_::AliasDb;
use collect::CollectDb;
pub(crate) use describe::DescribeDb;
pub(crate) use from_table::FromDb;
use group_by::GroupByDb;
pub(crate) use into_db::ToDataBase;
use into_sqlite::IntoSqliteDb;
use join::JoinDb;
use limit::LimitDb;
use nu_protocol::engine::StateWorkingSet;
use open_db::OpenDb;
use or::OrDb;
use order_by::OrderByDb;
use query_db::QueryDb;
use schema::SchemaDb;
pub(crate) use select::ProjectionDb;
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
        IntoSqliteDb,
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
