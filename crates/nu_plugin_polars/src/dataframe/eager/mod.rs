mod append;
mod cast;
mod columns;
mod drop;
mod drop_duplicates;
mod drop_nulls;
mod dummies;
mod filter_with;
mod first;
mod get;
mod last;
mod open;
mod pivot;
mod query_df;
mod rename;
mod sample;
mod save;
mod schema;
mod shape;
mod slice;
mod sql_context;
mod sql_expr;
mod summary;
mod take;
mod to_df;
mod to_nu;
mod unpivot;
mod with_column;

use crate::PolarsPlugin;

pub use self::open::OpenDataFrame;
pub use append::AppendDF;
pub use cast::CastDF;
pub use columns::ColumnsDF;
pub use drop::DropDF;
pub use drop_duplicates::DropDuplicates;
pub use drop_nulls::DropNulls;
pub use dummies::Dummies;
pub use filter_with::FilterWith;
pub use first::FirstDF;
pub use get::GetDF;
pub use last::LastDF;
use nu_plugin::PluginCommand;
pub use query_df::QueryDf;
pub use rename::RenameDF;
pub use sample::SampleDF;
pub use schema::SchemaCmd;
pub use shape::ShapeDF;
pub use slice::SliceDF;
pub use sql_context::SQLContext;
pub use summary::Summary;
pub use take::TakeDF;
pub use to_df::ToDataFrame;
pub use to_nu::ToNu;
pub use unpivot::UnpivotDF;
pub use with_column::WithColumn;

pub(crate) fn eager_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(AppendDF),
        Box::new(CastDF),
        Box::new(ColumnsDF),
        Box::new(DropDF),
        Box::new(DropDuplicates),
        Box::new(DropNulls),
        Box::new(Dummies),
        Box::new(FilterWith),
        Box::new(GetDF),
        Box::new(OpenDataFrame),
        Box::new(pivot::PivotDF),
        Box::new(UnpivotDF),
        Box::new(Summary),
        Box::new(FirstDF),
        Box::new(LastDF),
        Box::new(RenameDF),
        Box::new(SampleDF),
        Box::new(ShapeDF),
        Box::new(SliceDF),
        Box::new(SchemaCmd),
        Box::new(TakeDF),
        Box::new(ToNu),
        Box::new(ToDataFrame),
        Box::new(QueryDf),
        Box::new(WithColumn),
        Box::new(save::SaveDF),
    ]
}
