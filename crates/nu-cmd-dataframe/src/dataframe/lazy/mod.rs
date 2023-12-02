pub mod aggregate;
mod collect;
mod explode;
mod fetch;
mod fill_nan;
mod fill_null;
mod filter;
mod flatten;
pub mod groupby;
mod join;
mod macro_commands;
mod quantile;
mod select;
mod sort_by_expr;
mod to_lazy;

pub use explode::LazyExplode;
pub use flatten::LazyFlatten;
use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::lazy::{
    aggregate::LazyAggregate, fetch::LazyFetch, fill_nan::LazyFillNA, fill_null::LazyFillNull,
    filter::LazyFilter, groupby::ToLazyGroupBy, join::LazyJoin, quantile::LazyQuantile,
    sort_by_expr::LazySortBy,
};
pub use crate::dataframe::lazy::{collect::LazyCollect, to_lazy::ToLazyFrame};
pub(crate) use crate::dataframe::lazy::{macro_commands::*, select::LazySelect};

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
        LazyMedian,
        LazyReverse,
        LazySelect,
        LazySortBy,
        ToLazyFrame,
        ToLazyGroupBy,
        LazyExplode,
        LazyFlatten
    );
}
