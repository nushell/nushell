pub mod nu_dataframe;
pub mod nu_groupby;
pub mod operations;

pub use nu_dataframe::{Column, NuDataFrame};
pub use nu_groupby::NuGroupBy;
pub use operations::Axis;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum FrameStruct {
    GroupBy(NuGroupBy),
}
