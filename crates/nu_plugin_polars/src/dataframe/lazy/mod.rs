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

use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::lazy::aggregate::LazyAggregate;
pub use crate::dataframe::lazy::collect::LazyCollect;
use crate::dataframe::lazy::fetch::LazyFetch;
use crate::dataframe::lazy::fill_nan::LazyFillNA;
pub use crate::dataframe::lazy::fill_null::LazyFillNull;
use crate::dataframe::lazy::filter::LazyFilter;
use crate::dataframe::lazy::groupby::ToLazyGroupBy;
use crate::dataframe::lazy::join::LazyJoin;
pub(crate) use crate::dataframe::lazy::macro_commands::*;
use crate::dataframe::lazy::quantile::LazyQuantile;
pub(crate) use crate::dataframe::lazy::select::LazySelect;
use crate::dataframe::lazy::sort_by_expr::LazySortBy;
pub use crate::dataframe::lazy::to_lazy::ToLazyFrame;
pub use explode::LazyExplode;
pub use flatten::LazyFlatten;

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
