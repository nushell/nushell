pub mod nu_dataframe;
pub mod nu_groupby;

pub use nu_dataframe::{Column, NuDataFrame};
pub use nu_groupby::NuGroupBy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum FrameStruct {
    GroupBy(NuGroupBy),
}
