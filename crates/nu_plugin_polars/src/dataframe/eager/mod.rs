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
mod pivot;
mod query_df;
mod rename;
mod sample;
mod slice;
mod sql_context;
mod sql_expr;
mod take;
mod unpivot;
mod with_column;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

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
pub use query_df::QueryDf;
pub use rename::RenameDF;
pub use sample::SampleDF;
pub use slice::SliceDF;
pub use sql_context::SQLContext;
pub use take::TakeDF;
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
        Box::new(pivot::PivotDF),
        Box::new(UnpivotDF),
        Box::new(FirstDF),
        Box::new(LastDF),
        Box::new(RenameDF),
        Box::new(SampleDF),
        Box::new(SliceDF),
        Box::new(TakeDF),
        Box::new(QueryDf),
        Box::new(WithColumn),
    ]
}
