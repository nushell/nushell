mod aggregate;
mod cast;
mod collect;
mod drop;
mod drop_duplicates;
mod drop_nulls;
mod explode;
mod fetch;
mod fill_nan;
mod fill_null;
mod filter;
mod filter_with;
mod first;
mod flatten;
mod get;
pub mod groupby;
mod join;
mod last;
mod macro_commands;
mod median;
mod melt;
mod quantile;
mod rename;
mod select;
mod slice;
mod sort_by_expr;
mod with_column;

use nu_plugin::PluginCommand;

pub use crate::dataframe::lazy::aggregate::LazyAggregate;
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
use crate::PolarsPlugin;
pub use explode::LazyExplode;
pub use flatten::LazyFlatten;

pub(crate) fn lazy_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(cast::CastDF),
        Box::new(drop::DropDF),
        Box::new(drop_duplicates::DropDuplicates),
        Box::new(drop_nulls::DropNulls),
        Box::new(filter_with::FilterWith),
        Box::new(first::FirstDF),
        Box::new(get::GetDF),
        Box::new(last::LastDF),
        Box::new(LazyAggregate),
        Box::new(LazyCache),
        Box::new(LazyCollect),
        Box::new(LazyExplode),
        Box::new(LazyFetch),
        Box::new(LazyFillNA),
        Box::new(LazyFillNull),
        Box::new(LazyFilter),
        Box::new(LazyFlatten),
        Box::new(LazyJoin),
        Box::new(median::LazyMedian),
        Box::new(melt::MeltDF),
        Box::new(LazyReverse),
        Box::new(LazySelect),
        Box::new(LazySortBy),
        Box::new(LazyQuantile),
        Box::new(rename::RenameDF),
        Box::new(slice::SliceDF),
        Box::new(ToLazyGroupBy),
        Box::new(with_column::WithColumn),
    ]
}
