pub mod nu_dataframe;
pub mod nu_groupby;

pub use nu_dataframe::NuDataFrame;
pub use nu_groupby::NuGroupBy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum PolarsData {
    EagerDataFrame(NuDataFrame),
    GroupBy(NuGroupBy),
}
