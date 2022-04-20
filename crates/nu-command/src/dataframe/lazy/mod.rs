mod into_expression;

mod aggregate;
mod collect;
mod command;
mod fetch;
mod fill_na;
mod fill_null;
mod filter;
mod groupby;
mod join;
mod macro_commands;
mod quantile;
mod rename;
mod select;
mod shift;
mod sort_by_expr;
mod to_lazy;
mod with_column;

use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::lazy::macro_commands::*;

use crate::dataframe::lazy::aggregate::LazyAggregate;
use crate::dataframe::lazy::collect::LazyCollect;
use crate::dataframe::lazy::command::LazyDataframe;
use crate::dataframe::lazy::fetch::LazyFetch;
use crate::dataframe::lazy::fill_na::LazyFillNA;
use crate::dataframe::lazy::fill_null::LazyFillNull;
use crate::dataframe::lazy::filter::LazyFilter;
use crate::dataframe::lazy::groupby::ToLazyGroupBy;
use crate::dataframe::lazy::join::LazyJoin;
use crate::dataframe::lazy::quantile::LazyQuantile;
use crate::dataframe::lazy::rename::LazyRename;
use crate::dataframe::lazy::select::LazySelect;
use crate::dataframe::lazy::shift::LazyShift;
use crate::dataframe::lazy::sort_by_expr::LazySortBy;
use crate::dataframe::lazy::to_lazy::ToLazyFrame;
use crate::dataframe::lazy::with_column::LazyWithColumn;

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
        LazyCache,
        LazyCollect,
        LazyDataframe,
        LazyFetch,
        LazyFillNA,
        LazyFillNull,
        LazyFilter,
        LazyJoin,
        LazyQuantile,
        LazyMax,
        LazyMin,
        LazySum,
        LazyMean,
        LazyMedian,
        LazyStd,
        LazyVar,
        LazyRename,
        LazyReverse,
        LazySelect,
        LazyShift,
        LazySortBy,
        LazyWithColumn,
        ToLazyFrame,
        ToLazyGroupBy
    );
}
