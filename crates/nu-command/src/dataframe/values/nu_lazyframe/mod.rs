mod custom_value;

use polars::prelude::LazyFrame;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NuLazyFrame(LazyFrame);

impl NuLazyFrame {
    pub fn new(lazyframe: LazyFrame) -> Self {
        NuLazyFrame(lazyframe)
    }
}

