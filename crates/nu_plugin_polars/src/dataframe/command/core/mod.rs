mod cache;
mod columns;
mod fetch;
mod open;
mod profile;
mod resource;
mod save;
mod schema;
mod shape;
mod summary;
mod to_df;
mod to_lazy;
mod to_nu;
mod to_repr;

pub use self::open::OpenDataFrame;
use crate::PolarsPlugin;
use fetch::LazyFetch;
use nu_plugin::PluginCommand;
pub use schema::SchemaCmd;
pub use shape::ShapeDF;
pub use summary::Summary;
pub use to_df::ToDataFrame;
pub use to_lazy::ToLazyFrame;
pub use to_nu::ToNu;
pub use to_repr::ToRepr;

pub(crate) fn core_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(columns::ColumnsDF),
        Box::new(cache::LazyCache),
        Box::new(LazyFetch),
        Box::new(OpenDataFrame),
        Box::new(profile::ProfileDF),
        Box::new(Summary),
        Box::new(ShapeDF),
        Box::new(SchemaCmd),
        Box::new(ToNu),
        Box::new(ToDataFrame),
        Box::new(save::SaveDF),
        Box::new(ToLazyFrame),
        Box::new(ToRepr),
    ]
}
