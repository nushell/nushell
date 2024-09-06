mod collect;
mod explode;
mod fill_nan;
mod fill_null;
mod filter;
mod flatten;
mod join;
mod reverse;
mod select;
mod sort_by_expr;

use nu_plugin::PluginCommand;

pub use crate::dataframe::lazy::collect::LazyCollect;
use crate::dataframe::lazy::fill_nan::LazyFillNA;
pub use crate::dataframe::lazy::fill_null::LazyFillNull;
use crate::dataframe::lazy::filter::LazyFilter;
use crate::dataframe::lazy::join::LazyJoin;
use crate::dataframe::lazy::sort_by_expr::LazySortBy;
use crate::PolarsPlugin;
pub use explode::LazyExplode;
pub use flatten::LazyFlatten;

pub(crate) fn lazy_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(LazyCollect),
        Box::new(LazyExplode),
        Box::new(LazyFillNA),
        Box::new(LazyFillNull),
        Box::new(LazyFilter),
        Box::new(LazyFlatten),
        Box::new(LazyJoin),
        Box::new(reverse::LazyReverse),
        Box::new(select::LazySelect),
        Box::new(LazySortBy),
    ]
}
