mod into_expression;

mod aggregate;
mod collect;
mod command;
mod fetch;
mod filter;
mod groupby;
mod join;
mod reverse;
mod sort_by_expr;
mod to_lazy;
mod with_column;
mod open_csv;

use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::lazy::aggregate::LazyAggregate;
use crate::dataframe::lazy::collect::LazyCollect;
use crate::dataframe::lazy::command::LazyDataframe;
use crate::dataframe::lazy::fetch::LazyFetch;
use crate::dataframe::lazy::filter::LazyFilter;
use crate::dataframe::lazy::groupby::ToLazyGroupBy;
use crate::dataframe::lazy::join::LazyJoin;
use crate::dataframe::lazy::reverse::LazyReverse;
use crate::dataframe::lazy::sort_by_expr::LazySortBy;
use crate::dataframe::lazy::to_lazy::ToLazyFrame;
use crate::dataframe::lazy::with_column::LazyWithColumn;
use crate::dataframe::lazy::open_csv::LazyOpenCSV;

pub fn add_lazy_decls(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    // Dataframe commands
    bind_command!(
        LazyAggregate,
        LazyCollect,
        LazyDataframe,
        LazyFetch,
        LazyFilter,
        LazyJoin,
        LazyReverse,
        LazyOpenCSV,
        LazySortBy,
        LazyWithColumn,
        ToLazyFrame,
        ToLazyGroupBy
    );
}
