mod get;
mod list;
mod rm;

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use chrono::{DateTime, FixedOffset, Local};
pub use list::ListDF;
use nu_plugin::{EngineInterface, PluginCommand};
use nu_protocol::{LabeledError, ShellError, Span};
use uuid::Uuid;

use crate::{EngineWrapper, PolarsPlugin, values::PolarsPluginObject};

use log::debug;

#[derive(Debug, Clone)]
pub struct CacheValue {
    pub uuid: Uuid,
    pub value: PolarsPluginObject,
    pub created: DateTime<FixedOffset>,
    pub span: Span,
    pub reference_count: i16,
}

#[derive(Default)]
pub struct Cache {
    cache: Mutex<HashMap<Uuid, CacheValue>>,
}

impl Cache {
    fn lock(&self) -> Result<MutexGuard<'_, HashMap<Uuid, CacheValue>>, ShellError> {
        self.cache.lock().map_err(|e| ShellError::GenericError {
            error: format!("error acquiring cache lock: {e}"),
            msg: "".into(),
            span: None,
            help: None,
            inner: vec![],
        })
    }

    /// Removes an item from the plugin cache.
    ///
    /// * `maybe_engine` - Current EngineInterface reference. Required outside of testing
    /// * `key` - The key of the cache entry to remove.
    /// * `force` - Delete even if there are multiple references
    pub fn remove(
        &self,
        engine: impl EngineWrapper,
        key: &Uuid,
        force: bool,
    ) -> Result<Option<CacheValue>, ShellError> {
        let mut lock = self.lock()?;

        let reference_count = lock.get_mut(key).map(|cache_value| {
            cache_value.reference_count -= 1;
            cache_value.reference_count
        });

        let removed = if force || reference_count.unwrap_or_default() < 1 {
            let removed = lock.remove(key);
            debug!("PolarsPlugin: removing {key} from cache: {removed:?}");
            removed
        } else {
            debug!("PolarsPlugin: decrementing reference count for {key}");
            None
        };

        if lock.is_empty() {
            // Once there are no more entries in the cache
            // we can turn plugin gc back on
            debug!("PolarsPlugin: Cache is empty enabling GC");
            engine.set_gc_disabled(false).map_err(LabeledError::from)?;
        }
        drop(lock);
        Ok(removed)
    }

    /// Inserts an item into the plugin cache.
    /// The maybe_engine parameter is required outside of testing
    pub fn insert(
        &self,
        engine: impl EngineWrapper,
        uuid: Uuid,
        value: PolarsPluginObject,
        span: Span,
    ) -> Result<Option<CacheValue>, ShellError> {
        let mut lock = self.lock()?;
        debug!("PolarsPlugin: Inserting {uuid} into cache: {value:?}");
        // turn off plugin gc the first time an entry is added to the cache
        // as we don't want the plugin to be garbage collected if there
        // is any live data
        debug!("PolarsPlugin: Cache has values disabling GC");
        engine.set_gc_disabled(true).map_err(LabeledError::from)?;
        let cache_value = CacheValue {
            uuid,
            value,
            created: Local::now().into(),
            span,
            reference_count: 1,
        };
        let result = lock.insert(uuid, cache_value);
        drop(lock);
        Ok(result)
    }

    pub fn get(&self, uuid: &Uuid, increment: bool) -> Result<Option<CacheValue>, ShellError> {
        let mut lock = self.lock()?;
        let result = lock.get_mut(uuid).map(|cv| {
            if increment {
                cv.reference_count += 1;
            }
            cv.clone()
        });
        drop(lock);
        Ok(result)
    }

    pub fn process_entries<F, T>(&self, mut func: F) -> Result<Vec<T>, ShellError>
    where
        F: FnMut((&Uuid, &CacheValue)) -> Result<T, ShellError>,
    {
        let lock = self.lock()?;
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

    fn cache(
        self,
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
        span: Span,
    ) -> Result<Self, ShellError> {
        plugin.cache.insert(
            engine,
            self.cache_id().to_owned(),
            self.to_cache_value()?,
            span,
        )?;
        Ok(self)
    }

    fn get_cached(plugin: &PolarsPlugin, id: &Uuid) -> Result<Option<Self>, ShellError> {
        if let Some(cache_value) = plugin.cache.get(id, false)? {
            Ok(Some(Self::from_cache_value(cache_value.value)?))
        } else {
            Ok(None)
        }
    }
}

pub(crate) fn cache_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(ListDF),
        Box::new(rm::CacheRemove),
        Box::new(get::CacheGet),
    ]
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    struct MockEngineWrapper {
        gc_enabled: Rc<RefCell<bool>>,
    }

    impl MockEngineWrapper {
        fn new(gc_enabled: bool) -> Self {
            Self {
                gc_enabled: Rc::new(RefCell::new(gc_enabled)),
            }
        }

        fn gc_enabled(&self) -> bool {
            *self.gc_enabled.borrow()
        }
    }

    impl EngineWrapper for &MockEngineWrapper {
        fn get_env_var(&self, _key: &str) -> Option<String> {
            unimplemented!()
        }

        fn use_color(&self) -> bool {
            unimplemented!()
        }

        fn set_gc_disabled(&self, disabled: bool) -> Result<(), ShellError> {
            let _ = self.gc_enabled.replace(!disabled);
            Ok(())
        }
    }

    #[test]
    pub fn test_remove_plugin_cache_enable() {
        let mock_engine = MockEngineWrapper::new(false);

        let cache = Cache::default();
        let mut lock = cache.cache.lock().expect("should be able to acquire lock");
        let key0 = Uuid::new_v4();
        lock.insert(
            key0,
            CacheValue {
                uuid: Uuid::new_v4(),
                value: PolarsPluginObject::NuPolarsTestData(Uuid::new_v4(), "object_0".into()),
                created: Local::now().into(),
                span: Span::unknown(),
                reference_count: 1,
            },
        );

        let key1 = Uuid::new_v4();
        lock.insert(
            key1,
            CacheValue {
                uuid: Uuid::new_v4(),
                value: PolarsPluginObject::NuPolarsTestData(Uuid::new_v4(), "object_1".into()),
                created: Local::now().into(),
                span: Span::unknown(),
                reference_count: 1,
            },
        );
        drop(lock);

        let _ = cache
            .remove(&mock_engine, &key0, false)
            .expect("should be able to remove key0");
        assert!(!mock_engine.gc_enabled());

        let _ = cache
            .remove(&mock_engine, &key1, false)
            .expect("should be able to remove key1");
        assert!(mock_engine.gc_enabled());
    }
}
