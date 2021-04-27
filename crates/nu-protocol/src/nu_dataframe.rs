use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use polars::prelude::DataFrame;
use serde::de::{Deserialize, Deserializer, Visitor};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub struct NuDataFrame {
    #[serde(skip_serializing, default)]
    pub dataframe: Option<DataFrame>,
}

impl Default for NuDataFrame {
    fn default() -> Self {
        NuDataFrame { dataframe: None }
    }
}

impl NuDataFrame {
    fn new() -> Self {
        Self::default()
    }
}

impl PartialEq for NuDataFrame {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Eq for NuDataFrame {}

impl PartialOrd for NuDataFrame {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl Ord for NuDataFrame {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl Hash for NuDataFrame {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

impl<'de> Visitor<'de> for NuDataFrame {
    type Value = Self;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }
}

impl<'de> Deserialize<'de> for NuDataFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_i32(NuDataFrame::new())
    }
}
