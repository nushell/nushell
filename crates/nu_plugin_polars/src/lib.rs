use dataframe::values::{
    NuDataFrame, NuDataFrameCustomValue, NuExpression, NuLazyFrame, NuLazyFrameCustomValue,
    NuLazyGroupBy,
};
use lazy_static::lazy_static;
use nu_plugin::{EngineInterface, Plugin, PluginCommand};

pub mod dataframe;
pub use dataframe::*;
use nu_protocol::{CustomValue, LabeledError, ShellError};
use std::{collections::BTreeMap, sync::Mutex};
use uuid::Uuid;

use crate::{
    eager::{
        AppendDF, CastDF, ColumnsDF, DataTypes, DropDF, DropDuplicates, FirstDF, LastDF, ListDF,
        OpenDataFrame, Summary, ToArrow, ToCSV, ToDataFrame, ToNu, ToParquet,
    },
    expressions::{
        ExprAggGroups, ExprCount, ExprList, ExprMax, ExprMean, ExprMedian, ExprMin, ExprNot,
        ExprStd, ExprSum, ExprVar,
    },
    lazy::{LazyAggregate, LazyCache, LazyCollect, LazyMedian, LazyReverse},
};

lazy_static! {
    static ref CACHE: Mutex<BTreeMap<Uuid, CacheValue>> = Mutex::new(BTreeMap::new());
}

#[derive(Debug, Clone)]
pub(crate) enum CacheValue {
    DataFrame(NuDataFrame),
    LazyFrame(NuLazyFrame),
    LazyGroupBy(NuLazyGroupBy),
    Expression(NuExpression),
}

pub(crate) struct DataFrameCache;

impl DataFrameCache {
    pub(crate) fn remove(
        engine: &EngineInterface,
        uuid: &Uuid,
    ) -> Result<Option<CacheValue>, ShellError> {
        let mut lock = CACHE.try_lock().map_err(|e| ShellError::GenericError {
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

    fn insert(
        engine: &EngineInterface,
        uuid: Uuid,
        value: CacheValue,
    ) -> Result<Option<CacheValue>, ShellError> {
        let mut lock = CACHE.try_lock().map_err(|e| ShellError::GenericError {
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

    fn get(uuid: &Uuid) -> Result<Option<CacheValue>, ShellError> {
        let lock = CACHE.try_lock().map_err(|e| ShellError::GenericError {
            error: format!("error getting id {uuid} from cache: {e}"),
            msg: "".into(),
            span: None,
            help: None,
            inner: vec![],
        })?;
        Ok(lock.get(uuid).cloned())
    }

    pub(crate) fn process_entries<F, T>(mut func: F) -> Result<Vec<T>, ShellError>
    where
        F: FnMut((&Uuid, &CacheValue)) -> Result<T, ShellError>,
    {
        let lock = CACHE.try_lock().map_err(|e| ShellError::GenericError {
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

    pub(crate) fn insert_df(engine: &EngineInterface, df: NuDataFrame) -> Result<(), ShellError> {
        eprintln!("Adding dataframe to cache: {:?}", df.id);
        Self::insert(engine, df.id, CacheValue::DataFrame(df)).map(|_| ())
    }

    pub(crate) fn get_df(uuid: &Uuid) -> Result<Option<NuDataFrame>, ShellError> {
        Self::get(uuid).and_then(|get| match get {
            Some(CacheValue::DataFrame(df)) => Ok(Some(df)),
            v @ Some(_) => Err(ShellError::GenericError {
                error: format!("Cache key {uuid} is not a NuDataFrame: {v:?}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
            _ => Ok(None),
        })
    }

    pub(crate) fn insert_lazy(
        engine: &EngineInterface,
        lazy: NuLazyFrame,
    ) -> Result<(), ShellError> {
        eprintln!("Adding lazy dataframe to cache: {:?}", lazy.id);
        Self::insert(engine, lazy.id, CacheValue::LazyFrame(lazy)).map(|_| ())
    }

    pub(crate) fn get_lazy(uuid: &Uuid) -> Result<Option<NuLazyFrame>, ShellError> {
        Self::get(uuid).and_then(|get| match get {
            Some(CacheValue::LazyFrame(df)) => Ok(Some(df)),
            v @ Some(_) => Err(ShellError::GenericError {
                error: format!("Cache key {uuid} is not a NuLazyFrame: {v:?}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
            _ => Ok(None),
        })
    }

    pub(crate) fn insert_group_by(
        engine: &EngineInterface,
        group_by: NuLazyGroupBy,
    ) -> Result<(), ShellError> {
        eprintln!("Adding lazy groupby to cache: {:?}", group_by.id);
        Self::insert(engine, group_by.id, CacheValue::LazyGroupBy(group_by)).map(|_| ())
    }

    pub(crate) fn get_group_by(uuid: &Uuid) -> Result<Option<NuLazyGroupBy>, ShellError> {
        Self::get(uuid).and_then(|get| match get {
            Some(CacheValue::LazyGroupBy(group_by)) => Ok(Some(group_by)),
            v @ Some(_) => Err(ShellError::GenericError {
                error: format!("Cache value {uuid} - {v:?} is not a LazyGroupBy"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
            _ => Ok(None),
        })
    }

    pub(crate) fn insert_expr(
        engine: &EngineInterface,
        expr: NuExpression,
    ) -> Result<(), ShellError> {
        eprintln!("Adding expr to cache: {:?}", expr.id);
        Self::insert(engine, expr.id, CacheValue::Expression(expr)).map(|_| ())
    }

    pub(crate) fn get_expr(uuid: &Uuid) -> Result<Option<NuExpression>, ShellError> {
        Self::get(uuid).and_then(|get| match get {
            Some(CacheValue::Expression(expr)) => Ok(Some(expr)),
            v @ Some(_) => Err(ShellError::GenericError {
                error: format!("Cache value {uuid} - {v:?} is not a LazyGroupBy"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
            _ => Ok(None),
        })
    }
}

pub struct PolarsDataFramePlugin;

impl Plugin for PolarsDataFramePlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(AppendDF),
            Box::new(CastDF),
            Box::new(DataTypes),
            Box::new(DropDF),
            Box::new(DropDuplicates),
            Box::new(OpenDataFrame),
            Box::new(ToDataFrame),
            Box::new(Summary),
            Box::new(FirstDF),
            Box::new(LastDF),
            Box::new(ListDF),
            Box::new(ColumnsDF),
            Box::new(ToNu),
            Box::new(ToArrow),
            Box::new(ToCSV),
            Box::new(ToParquet),
            Box::new(LazyAggregate),
            Box::new(LazyCache),
            Box::new(LazyCollect),
            Box::new(LazyMedian),
            Box::new(LazyReverse),
            Box::new(ExprList),
            Box::new(ExprAggGroups),
            Box::new(ExprCount),
            Box::new(ExprNot),
            Box::new(ExprMax),
            Box::new(ExprMin),
            Box::new(ExprSum),
            Box::new(ExprMean),
            Box::new(ExprMedian),
            Box::new(ExprStd),
            Box::new(ExprVar),
        ]
    }

    fn custom_value_dropped(
        &self,
        engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
    ) -> Result<(), LabeledError> {
        let any = custom_value.as_any();

        let maybe_id = if let Some(df) = any.downcast_ref::<NuDataFrameCustomValue>() {
            eprintln!("removing DataFrame id: {:?} from cache", df.id);
            Some(df.id)
        } else if let Some(lazy) = any.downcast_ref::<NuLazyFrameCustomValue>() {
            eprintln!("removing LazyFrame id: {:?} from cache", lazy.id);
            Some(lazy.id)
        } else if let Some(gb) = any.downcast_ref::<NuLazyGroupBy>() {
            eprintln!("removing GroupBy id: {:?} from cache", gb.id);
            Some(gb.id)
        } else {
            None
        };

        if let Some(ref id) = maybe_id {
            let _ = DataFrameCache::remove(engine, id)
                .map_err(|e: ShellError| LabeledError::from(e))?;
        }

        Ok(())
    }
}
