use serde::{Deserialize, Serialize};
pub mod nu_dataframe;

pub use nu_dataframe::NuDataFrame;

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum PolarsStruct {
    DataFrame(NuDataFrame),
    Series,
    GroupBy,
    LazyFrame,
    GroupTuples,
    None,
}
