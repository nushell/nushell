mod append;
mod columns;
mod describe;
mod drop;
mod drop_duplicates;
mod drop_nulls;
mod dtypes;
mod dummies;
mod filter_with;
mod first;
mod get;
mod last;
mod list;
mod melt;
mod open;
mod query_df;
mod rename;
mod sample;
mod shape;
mod slice;
mod sql_context;
mod sql_expr;
mod take;
mod to_arrow;
mod to_csv;
mod to_df;
mod to_nu;
mod to_parquet;
mod with_column;

use nu_protocol::engine::StateWorkingSet;

pub use append::AppendDF;
pub use columns::ColumnsDF;
pub use describe::DescribeDF;
pub use drop::DropDF;
pub use drop_duplicates::DropDuplicates;
pub use drop_nulls::DropNulls;
pub use dtypes::DataTypes;
pub use dummies::Dummies;
pub use filter_with::FilterWith;
pub use first::FirstDF;
pub use get::GetDF;
pub use last::LastDF;
pub use list::ListDF;
pub use melt::MeltDF;
pub use open::OpenDataFrame;
pub use query_df::QueryDf;
pub use rename::RenameDF;
pub use sample::SampleDF;
pub use shape::ShapeDF;
pub use slice::SliceDF;
pub use sql_context::SQLContext;
pub use sql_expr::parse_sql_expr;
pub use take::TakeDF;
pub use to_arrow::ToArrow;
pub use to_csv::ToCSV;
pub use to_df::ToDataFrame;
pub use to_nu::ToNu;
pub use to_parquet::ToParquet;
pub use with_column::WithColumn;

pub fn add_eager_decls(working_set: &mut StateWorkingSet) {
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
        AppendDF,
        ColumnsDF,
        DataTypes,
        DescribeDF,
        DropDF,
        DropDuplicates,
        DropNulls,
        Dummies,
        FilterWith,
        FirstDF,
        GetDF,
        LastDF,
        ListDF,
        MeltDF,
        OpenDataFrame,
        QueryDf,
        RenameDF,
        SampleDF,
        ShapeDF,
        SliceDF,
        TakeDF,
        ToArrow,
        ToCSV,
        ToDataFrame,
        ToNu,
        ToParquet,
        WithColumn
    );
}
