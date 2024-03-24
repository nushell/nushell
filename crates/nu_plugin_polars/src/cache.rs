use std::{collections::HashMap, sync::Mutex};

use nu_plugin::EngineInterface;
use nu_protocol::{LabeledError, ShellError};
use uuid::Uuid;

use crate::{values::PhysicalType, PolarsPlugin};

#[derive(Default)]
pub struct Cache {
    cache: Mutex<HashMap<Uuid, PhysicalType>>,
}

impl Cache {
    pub fn remove(
        &self,
        engine: &EngineInterface,
        uuid: &Uuid,
    ) -> Result<Option<PhysicalType>, ShellError> {
        let mut lock = self
            .cache
            .try_lock()
            .map_err(|e| ShellError::GenericError {
                error: format!("error removing id {uuid} from cache: {e}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })?;
        let removed = lock.remove(uuid);
        // Once there are no more entries in the cache
        // we can turn plugin gc back on
        if lock.is_empty() {
            engine.set_gc_disabled(false).map_err(LabeledError::from)?;
        }
        Ok(removed)
    }

    pub fn insert(
        &self,
        engine: &EngineInterface,
        uuid: Uuid,
        value: PhysicalType,
    ) -> Result<Option<PhysicalType>, ShellError> {
        let mut lock = self
            .cache
            .try_lock()
            .map_err(|e| ShellError::GenericError {
                error: format!("error inserting id {uuid} into cache: {e}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })?;
        // turn off plugin gc the first time an entry is added to the cache
        // as we don't want the plugin to be garbage collected if there
        // is any live data
        if lock.is_empty() {
            engine.set_gc_disabled(true).map_err(LabeledError::from)?;
        }
        Ok(lock.insert(uuid, value))
    }

    pub fn get(&self, uuid: &Uuid) -> Result<Option<PhysicalType>, ShellError> {
        let lock = self
            .cache
            .try_lock()
            .map_err(|e| ShellError::GenericError {
                error: format!("error getting id {uuid} from cache: {e}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })?;
        Ok(lock.get(uuid).cloned())
    }

    pub fn process_entries<F, T>(&self, mut func: F) -> Result<Vec<T>, ShellError>
    where
        F: FnMut((&Uuid, &PhysicalType)) -> Result<T, ShellError>,
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
        Ok(vals)
    }
}

pub trait Cacheable: Sized + Clone {
    fn cache_id(&self) -> &Uuid;

    fn to_cache_value(&self) -> Result<PhysicalType, ShellError>;

    fn from_cache_value(cv: PhysicalType) -> Result<Self, ShellError>;

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
