mod aggregate;
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
mod median;
mod quantile;
mod select;
mod sort_by_expr;
mod to_lazy;

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
pub use crate::dataframe::lazy::to_lazy::ToLazyFrame;
use crate::PolarsPlugin;
pub use explode::LazyExplode;
pub use flatten::LazyFlatten;

pub(crate) fn lazy_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
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
        Box::new(LazyReverse),
        Box::new(LazySelect),
        Box::new(LazySortBy),
        Box::new(LazyQuantile),
        Box::new(ToLazyFrame),
        Box::new(ToLazyGroupBy),
    ]
}
