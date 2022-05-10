// Conversions between value and sqlparser objects
pub mod conversions;

mod alias;
mod and;
mod col;
mod collect;
mod command;
mod describe;
mod from;
mod function;
mod group_by;
mod join;
mod limit;
mod open;
mod or;
mod order_by;
mod over;
mod query;
mod schema;
mod select;
mod where_;

// Temporal module to create Query objects
mod testing;
use testing::TestingDb;

use nu_protocol::engine::StateWorkingSet;

use alias::AliasExpr;
use and::AndDb;
use col::ColExpr;
use collect::CollectDb;
use command::Database;
use describe::DescribeDb;
use from::FromDb;
use function::FunctionExpr;
use group_by::GroupByDb;
use join::JoinDb;
use limit::LimitDb;
use open::OpenDb;
use or::OrDb;
use order_by::OrderByDb;
use over::OverExpr;
use query::QueryDb;
use schema::SchemaDb;
use select::ProjectionDb;
use where_::WhereDb;

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
    bind_command!(
        AliasExpr,
        AndDb,
        ColExpr,
        CollectDb,
        Database,
        DescribeDb,
        FromDb,
        FunctionExpr,
        GroupByDb,
        JoinDb,
        LimitDb,
        OpenDb,
        OrderByDb,
        OrDb,
        OverExpr,
        QueryDb,
        ProjectionDb,
        SchemaDb,
        TestingDb,
        WhereDb
    );
}
