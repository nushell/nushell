mod alias;
mod append;
mod arg_where;
mod cast;
mod col;
mod collect;
mod concat;
mod cut;
mod drop;
mod drop_duplicates;
mod drop_nulls;
mod dummies;
mod explode;
mod fill_nan;
mod fill_null;
mod filter;
mod filter_with;
mod first;
mod flatten;
mod get;
mod join;
mod join_where;
mod last;
mod len;
mod lit;
mod pivot;
mod qcut;
mod query_df;
mod rename;
mod reverse;
mod sample;
mod select;
mod slice;
mod sort_by_expr;
pub mod sql_context;
pub mod sql_expr;
mod struct_json_encode;
mod take;
mod unnest;
mod unpivot;
mod with_column;
use filter::LazyFilter;
mod replace;
mod shift;
mod unique;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use alias::ExprAlias;
pub use append::AppendDF;
pub use arg_where::ExprArgWhere;
pub use cast::CastDF;
pub use col::ExprCol;
pub use collect::LazyCollect;
pub use drop::DropDF;
pub use drop_duplicates::DropDuplicates;
pub use drop_nulls::DropNulls;
pub use dummies::Dummies;
pub use explode::LazyExplode;
use fill_nan::LazyFillNA;
pub use fill_null::LazyFillNull;
pub use first::FirstDF;
use flatten::LazyFlatten;
pub use get::GetDF;
use join::LazyJoin;
use join_where::LazyJoinWhere;
pub use last::LastDF;
pub use lit::ExprLit;
use query_df::QueryDf;
pub use rename::RenameDF;
pub use replace::Replace;
pub use sample::SampleDF;
pub use shift::Shift;
pub use slice::SliceDF;
use sort_by_expr::LazySortBy;
pub use take::TakeDF;
pub use unique::Unique;
pub use with_column::WithColumn;

pub(crate) fn data_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(AppendDF),
        Box::new(CastDF),
        Box::new(cut::CutSeries),
        Box::new(DropDF),
        Box::new(concat::ConcatDF),
        Box::new(DropDuplicates),
        Box::new(DropNulls),
        Box::new(Dummies),
        Box::new(filter_with::FilterWith),
        Box::new(GetDF),
        Box::new(pivot::PivotDF),
        Box::new(unpivot::Unpivot),
        Box::new(FirstDF),
        Box::new(LastDF),
        Box::new(len::ExprLen),
        Box::new(RenameDF),
        Box::new(SampleDF),
        Box::new(SliceDF),
        Box::new(TakeDF),
        Box::new(QueryDf),
        Box::new(WithColumn),
        Box::new(ExprAlias),
        Box::new(ExprArgWhere),
        Box::new(ExprLit),
        Box::new(ExprCol),
        Box::new(LazyCollect),
        Box::new(LazyExplode),
        Box::new(LazyFillNA),
        Box::new(LazyFillNull),
        Box::new(LazyFlatten),
        Box::new(LazyJoin),
        Box::new(LazyJoinWhere),
        Box::new(reverse::LazyReverse),
        Box::new(select::LazySelect),
        Box::new(LazySortBy),
        Box::new(LazyFilter),
        Box::new(Replace),
        Box::new(Shift),
        Box::new(struct_json_encode::StructJsonEncode),
        Box::new(qcut::QCutSeries),
        Box::new(Unique),
        Box::new(unnest::UnnestDF),
    ]
}
