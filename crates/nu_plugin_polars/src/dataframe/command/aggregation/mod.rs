mod agg_groups;
mod aggregate;
mod count;
mod cumulative;
pub mod groupby;
mod implode;
mod max;
mod mean;
mod median;
mod min;
mod n_null;
mod n_unique;
mod quantile;
mod rolling;
mod std;
mod sum;
mod value_counts;
mod var;

use crate::PolarsPlugin;
use agg_groups::ExprAggGroups;
use nu_plugin::PluginCommand;

pub use aggregate::LazyAggregate;
use count::ExprCount;
pub use cumulative::Cumulative;
use implode::ExprImplode;
use max::ExprMax;
use mean::ExprMean;
use min::ExprMin;
pub use n_null::NNull;
pub use n_unique::NUnique;
pub use rolling::Rolling;
use std::ExprStd;
pub use sum::ExprSum;
pub use value_counts::ValueCount;
use var::ExprVar;

pub(crate) fn aggregation_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(Cumulative),
        Box::new(ExprAggGroups),
        Box::new(ExprCount),
        Box::new(ExprImplode),
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
