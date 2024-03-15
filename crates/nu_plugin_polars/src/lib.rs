use dashmap::DashMap;
use dataframe::values::{NuDataFrame, NuLazyFrame, NuLazyFrameCustomValue};
use lazy_static::lazy_static;
use nu_plugin::{EngineInterface, LabeledError, Plugin, PluginCommand};

pub mod dataframe;
pub use dataframe::values::NuDataFrameCustomValue;
pub use dataframe::*;
use nu_protocol::CustomValue;
use std::sync::Arc;
use uuid::Uuid;

use crate::eager::OpenDataFrame;

lazy_static! {
    static ref DATAFRAME_CACHE: Arc<DataFrameCache> = Arc::new(DataFrameCache::new());
}

pub(crate) enum CacheValue {
    DataFrame(NuDataFrame),
    LazyFrame(NuLazyFrame),
} 

pub(crate) struct DataFrameCache {
    internal: DashMap<Uuid, CacheValue>,
}

impl DataFrameCache {
    fn new() -> Self {
        Self {
            internal: DashMap::new(),
        }
    }

    pub(crate) fn remove(&self, uuid: &Uuid) -> Option<CacheValue> {
        self.internal.remove(uuid).map(|(_, v)| v)
    }

    pub(crate) fn insert_df(&self, df: NuDataFrame) {
        let _ = self.internal.insert(df.id, CacheValue::DataFrame(df));
    }

    pub(crate) fn insert_lazy(&self, lazy: NuLazyFrame) {
        let _ = self.internal.insert(lazy.id, CacheValue::LazyFrame(lazy));
    }

    pub fn instance() -> Arc<DataFrameCache> {
        Arc::clone(&DATAFRAME_CACHE)
    }
}

pub struct PolarsDataFramePlugin;

impl Plugin for PolarsDataFramePlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(OpenDataFrame)]
    }

    fn custom_value_dropped(
        &self,
        _engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        let any = custom_value.as_any();

        if let Some(df) = any.downcast_ref::<NuDataFrameCustomValue>() {
            eprintln!("removing id: {:?} from cache", df.id);
            DATAFRAME_CACHE.remove(&df.id);
        } else if let Some(lazy) = any.downcast_ref::<NuLazyFrameCustomValue>() {
            eprintln!("removing id: {:?} from cache", lazy.id);
            DATAFRAME_CACHE.remove(&lazy.id);
        }
        Ok(())
    }
}
