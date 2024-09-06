mod aggregate;
mod aggregation_commands;
mod cumulative;
pub mod groupby;
mod median;
mod n_null;
mod n_unique;
mod quantile;
mod rolling;
mod value_counts;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use aggregate::LazyAggregate;
pub use aggregation_commands::*;
pub use cumulative::Cumulative;
pub use n_null::NNull;
pub use n_unique::NUnique;
pub use rolling::Rolling;
pub use value_counts::ValueCount;

pub(crate) fn aggregation_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(Cumulative),
        Box::new(ExprAggGroups),
        Box::new(ExprCount),
        Box::new(ExprList),
        Box::new(ExprMax),
        Box::new(ExprMin),
        Box::new(ExprSum),
        Box::new(ExprMean),
        Box::new(ExprStd),
        Box::new(ExprVar),
        Box::new(LazyAggregate),
        Box::new(median::LazyMedian),
        Box::new(quantile::LazyQuantile),
        Box::new(groupby::ToLazyGroupBy),
        Box::new(Rolling),
        Box::new(ValueCount),
        Box::new(NNull),
        Box::new(NUnique),
    ]
}
