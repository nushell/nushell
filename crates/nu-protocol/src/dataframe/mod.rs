pub mod compute_between;
pub mod nu_dataframe;
pub mod nu_groupby;
pub mod operations;

pub use compute_between::{compute_between_dataframes, compute_series_single_value};
pub use nu_dataframe::{Column, NuDataFrame};
pub use nu_groupby::NuGroupBy;
pub use operations::Axis;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum FrameStruct {
    GroupBy(NuGroupBy),
}
