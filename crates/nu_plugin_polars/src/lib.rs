use dashmap::DashMap;
use dataframe::values::NuDataFrame;
use lazy_static::lazy_static;
use nu_plugin::{EngineInterface, LabeledError, Plugin, PluginCommand};

pub mod dataframe;
pub use dataframe::*;
use nu_protocol::CustomValue;
use std::sync::Arc;
use uuid::Uuid;

use crate::eager::OpenDataFrame;

lazy_static! {
    static ref DATAFRAME_CACHE: Arc<DataFrameCache> = Arc::new(DataFrameCache::new());
}

pub(crate) struct DataFrameCache {
    internal: DashMap<Uuid, NuDataFrame>,
}

impl DataFrameCache {
    fn new() -> Self {
        Self {
            internal: DashMap::new(),
        }
    }

    pub(crate) fn remove(&self, uuid: &Uuid) -> Option<NuDataFrame> {
        self.internal.remove(uuid).map(|(_, v)| v)
    }

    pub(crate) fn insert(&self, df: NuDataFrame) {
        let _ = self.internal.insert(df.id, df);
    }

    pub fn instance() -> Arc<DataFrameCache> {
        Arc::clone(&DATAFRAME_CACHE)
    }
}

pub struct PolarsDataframePlugin;

impl Plugin for PolarsDataframePlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(OpenDataFrame)]
    }

    fn custom_value_dropped(
        &self,
        _engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        if let Some(df) = custom_value.as_any().downcast_ref::<NuDataFrame>() {
            eprintln!("removing id: {:?} from cache", df.id);
            DATAFRAME_CACHE.remove(&df.id);
        }
        Ok(())
    }
}
