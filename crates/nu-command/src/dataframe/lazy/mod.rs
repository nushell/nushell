pub mod aggregate;
mod collect;
mod fetch;
mod fill_na;
mod fill_null;
mod filter;
pub mod groupby;
mod join;
mod macro_commands;
mod quantile;
mod select;
mod sort_by_expr;
mod to_lazy;

use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::lazy::aggregate::LazyAggregate;
pub use crate::dataframe::lazy::collect::LazyCollect;
use crate::dataframe::lazy::fetch::LazyFetch;
use crate::dataframe::lazy::fill_na::LazyFillNA;
use crate::dataframe::lazy::fill_null::LazyFillNull;
use crate::dataframe::lazy::filter::LazyFilter;
use crate::dataframe::lazy::groupby::ToLazyGroupBy;
use crate::dataframe::lazy::join::LazyJoin;
pub(crate) use crate::dataframe::lazy::macro_commands::*;
use crate::dataframe::lazy::quantile::LazyQuantile;
pub(crate) use crate::dataframe::lazy::select::LazySelect;
use crate::dataframe::lazy::sort_by_expr::LazySortBy;
pub use crate::dataframe::lazy::to_lazy::ToLazyFrame;

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
        LazyReverse,
        LazySelect,
        LazySortBy,
        ToLazyFrame,
        ToLazyGroupBy
    );
}
