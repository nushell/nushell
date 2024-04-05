use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use nu_plugin::EngineInterface;
use nu_protocol::{LabeledError, ShellError};
use uuid::Uuid;

use crate::{plugin_debug, values::PolarsPluginObject, PolarsPlugin};

#[derive(Default)]
pub struct Cache {
    cache: Mutex<HashMap<Uuid, PolarsPluginObject>>,
}

impl Cache {
    fn lock(&self) -> Result<MutexGuard<HashMap<Uuid, PolarsPluginObject>>, ShellError> {
        self.cache.lock().map_err(|e| ShellError::GenericError {
            error: format!("error acquiring cache lock: {e}"),
            msg: "".into(),
            span: None,
            help: None,
            inner: vec![],
        })
    }

    pub fn remove(
        &self,
        engine: &EngineInterface,
        uuid: &Uuid,
    ) -> Result<Option<PolarsPluginObject>, ShellError> {
        let mut lock = self.lock()?;
        let removed = lock.remove(uuid);
        plugin_debug!("PolarsPlugin: removing {uuid} from cache: {removed:?}");
        // Once there are no more entries in the cache
        // we can turn plugin gc back on
        if lock.is_empty() {
            plugin_debug!("PolarsPlugin: Cache is empty enabling GC");
            engine.set_gc_disabled(false).map_err(LabeledError::from)?;
        }
        drop(lock);
        Ok(removed)
    }

    pub fn insert(
        &self,
        engine: &EngineInterface,
        uuid: Uuid,
        value: PolarsPluginObject,
    ) -> Result<Option<PolarsPluginObject>, ShellError> {
        let mut lock = self.lock()?;
        plugin_debug!("PolarsPlugin: Inserting {uuid} into cache: {value:?}");
        // turn off plugin gc the first time an entry is added to the cache
        // as we don't want the plugin to be garbage collected if there
        // is any live data
        if lock.is_empty() {
            plugin_debug!("PolarsPlugin: Cache has values disabling GC");
            engine.set_gc_disabled(true).map_err(LabeledError::from)?;
        }
        let result = lock.insert(uuid, value);
        drop(lock);
        Ok(result)
    }

    pub fn get(&self, uuid: &Uuid) -> Result<Option<PolarsPluginObject>, ShellError> {
        let lock = self.lock()?;
        let result = lock.get(uuid).cloned();
        drop(lock);
        Ok(result)
    }

    pub fn process_entries<F, T>(&self, mut func: F) -> Result<Vec<T>, ShellError>
    where
        F: FnMut((&Uuid, &PolarsPluginObject)) -> Result<T, ShellError>,
    {
        let lock = self
            .cache
            .try_lock()
            .map_err(|e| ShellError::GenericError {
                error: format!("error getting entries from cache: {e}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })?;

        let mut vals: Vec<T> = Vec::new();
        for entry in lock.iter() {
            let val = func(entry)?;
            vals.push(val);
        }
        drop(lock);
        Ok(vals)
    }
}

pub trait Cacheable: Sized + Clone {
    fn cache_id(&self) -> &Uuid;

    fn to_cache_value(&self) -> Result<PolarsPluginObject, ShellError>;

    fn from_cache_value(cv: PolarsPluginObject) -> Result<Self, ShellError>;

    fn cache(self, plugin: &PolarsPlugin, engine: &EngineInterface) -> Result<Self, ShellError> {
        plugin
            .cache
            .insert(engine, self.cache_id().to_owned(), self.to_cache_value()?)?;
        Ok(self)
    }

    fn get_cached(plugin: &PolarsPlugin, id: &Uuid) -> Result<Option<Self>, ShellError> {
        if let Some(cache_value) = plugin.cache.get(id)? {
            Ok(Some(Self::from_cache_value(cache_value)?))
        } else {
            Ok(None)
        }
    }
}
