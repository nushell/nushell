mod append;
mod columns;
mod dummies;
mod open;
mod query_df;
mod sample;
mod schema;
mod shape;
mod sql_context;
mod sql_expr;
mod summary;
mod take;
mod to_arrow;
mod to_avro;
mod to_csv;
mod to_df;
mod to_json_lines;
mod to_nu;
mod to_parquet;

use crate::PolarsPlugin;

pub use self::open::OpenDataFrame;
pub use append::AppendDF;
pub use columns::ColumnsDF;
pub use dummies::Dummies;
use nu_plugin::PluginCommand;
pub use query_df::QueryDf;
pub use sample::SampleDF;
pub use schema::SchemaCmd;
pub use shape::ShapeDF;
pub use sql_context::SQLContext;
pub use summary::Summary;
pub use take::TakeDF;
pub use to_arrow::ToArrow;
pub use to_avro::ToAvro;
pub use to_csv::ToCSV;
pub use to_df::ToDataFrame;
pub use to_json_lines::ToJsonLines;
pub use to_nu::ToNu;
pub use to_parquet::ToParquet;

pub(crate) fn eager_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(AppendDF),
        Box::new(ColumnsDF),
        Box::new(Dummies),
        Box::new(OpenDataFrame),
        Box::new(Summary),
        Box::new(SampleDF),
        Box::new(ShapeDF),
        Box::new(SchemaCmd),
        Box::new(TakeDF),
        Box::new(ToNu),
        Box::new(ToArrow),
        Box::new(ToAvro),
        Box::new(ToDataFrame),
        Box::new(ToCSV),
        Box::new(ToJsonLines),
        Box::new(ToParquet),
        Box::new(QueryDf),
    ]
}
