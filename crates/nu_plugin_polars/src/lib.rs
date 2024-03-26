use std::cmp::Ordering;

pub use cache::{Cache, Cacheable};
use dataframe::values::{
    CustomValueType, NuDataFrameCustomValue, NuLazyFrameCustomValue, NuLazyGroupBy,
};
use nu_plugin::{EngineInterface, Plugin, PluginCommand};

mod cache;
pub mod dataframe;
pub use dataframe::*;
use nu_protocol::{
    ast::Operator, CustomValue, LabeledError, PipelineData, ShellError, Span, Spanned, Value,
};
use uuid::Uuid;

use crate::{
    eager::{
        AppendDF, CastDF, ColumnsDF, DataTypes, DropDF, DropDuplicates, DropNulls, FirstDF, LastDF,
        ListDF, OpenDataFrame, Summary, ToArrow, ToCSV, ToDataFrame, ToNu, ToParquet,
    },
    expressions::{
        ExprAggGroups, ExprCount, ExprList, ExprMax, ExprMean, ExprMedian, ExprMin, ExprNot,
        ExprStd, ExprSum, ExprVar,
    },
    lazy::{LazyAggregate, LazyCache, LazyCollect, LazyMedian, LazyReverse},
};

#[derive(Default)]
pub struct PolarsPlugin {
    pub(crate) cache: Cache,
}

impl Plugin for PolarsPlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(AppendDF),
            Box::new(CastDF),
            Box::new(DataTypes),
            Box::new(DropDF),
            Box::new(DropDuplicates),
            Box::new(DropNulls),
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
            let _ = self
                .cache
                .remove(engine, id)
                .map_err(|e: ShellError| LabeledError::from(e))?;
        }

        Ok(())
    }

    fn custom_value_to_base_value(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
    ) -> Result<Value, LabeledError> {
        eprintln!("Polars plugin customn_value_to_base_value called");
        match CustomValueType::try_from_custom_value(custom_value.item)? {
            CustomValueType::NuDataFrame(cv) => cv
                .custom_value_to_base_value(self, engine)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyFrame(cv) => cv
                .custom_value_to_base_value(self, engine)
                .map_err(LabeledError::from),
            CustomValueType::NuExpression(cv) => cv
                .custom_value_to_base_value(self, engine)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyGroupBy(cv) => cv
                .custom_value_to_base_value(self, engine)
                .map_err(LabeledError::from),
            CustomValueType::NuWhen(cv) => cv
                .custom_value_to_base_value(self, engine)
                .map_err(LabeledError::from),
        }
    }

    fn custom_value_operation(
        &self,
        engine: &EngineInterface,
        left: Spanned<Box<dyn CustomValue>>,
        operator: Spanned<Operator>,
        right: Value,
    ) -> Result<Value, LabeledError> {
        match CustomValueType::try_from_custom_value(left.item)? {
            CustomValueType::NuDataFrame(cv) => cv
                .custom_value_operation(self, engine, left.span, operator, right)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyFrame(cv) => cv
                .custom_value_operation(self, engine, left.span, operator, right)
                .map_err(LabeledError::from),

            CustomValueType::NuExpression(cv) => cv
                .custom_value_operation(self, engine, left.span, operator, right)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyGroupBy(cv) => cv
                .custom_value_operation(self, engine, left.span, operator, right)
                .map_err(LabeledError::from),
            CustomValueType::NuWhen(cv) => cv
                .custom_value_operation(self, engine, left.span, operator, right)
                .map_err(LabeledError::from),
        }
    }

    fn custom_value_follow_path_int(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
        index: Spanned<usize>,
    ) -> Result<Value, LabeledError> {
        match CustomValueType::try_from_custom_value(custom_value.item)? {
            CustomValueType::NuDataFrame(cv) => cv
                .custom_value_follow_path_int(self, engine, custom_value.span, index)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyFrame(cv) => cv
                .custom_value_follow_path_int(self, engine, custom_value.span, index)
                .map_err(LabeledError::from),
            CustomValueType::NuExpression(cv) => cv
                .custom_value_follow_path_int(self, engine, custom_value.span, index)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyGroupBy(cv) => cv
                .custom_value_follow_path_int(self, engine, custom_value.span, index)
                .map_err(LabeledError::from),
            CustomValueType::NuWhen(cv) => cv
                .custom_value_follow_path_int(self, engine, custom_value.span, index)
                .map_err(LabeledError::from),
        }
    }

    fn custom_value_follow_path_string(
        &self,
        engine: &EngineInterface,
        custom_value: Spanned<Box<dyn CustomValue>>,
        column_name: Spanned<String>,
    ) -> Result<Value, LabeledError> {
        match CustomValueType::try_from_custom_value(custom_value.item)? {
            CustomValueType::NuDataFrame(cv) => cv
                .custom_value_follow_path_string(self, engine, custom_value.span, column_name)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyFrame(cv) => cv
                .custom_value_follow_path_string(self, engine, custom_value.span, column_name)
                .map_err(LabeledError::from),
            CustomValueType::NuExpression(cv) => cv
                .custom_value_follow_path_string(self, engine, custom_value.span, column_name)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyGroupBy(cv) => cv
                .custom_value_follow_path_string(self, engine, custom_value.span, column_name)
                .map_err(LabeledError::from),
            CustomValueType::NuWhen(cv) => cv
                .custom_value_follow_path_string(self, engine, custom_value.span, column_name)
                .map_err(LabeledError::from),
        }
    }

    fn custom_value_partial_cmp(
        &self,
        engine: &EngineInterface,
        custom_value: Box<dyn CustomValue>,
        other_value: Value,
    ) -> Result<Option<Ordering>, LabeledError> {
        match CustomValueType::try_from_custom_value(custom_value)? {
            CustomValueType::NuDataFrame(cv) => cv
                .custom_value_partial_cmp(self, engine, other_value)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyFrame(cv) => cv
                .custom_value_partial_cmp(self, engine, other_value)
                .map_err(LabeledError::from),
            CustomValueType::NuExpression(cv) => cv
                .custom_value_partial_cmp(self, engine, other_value)
                .map_err(LabeledError::from),
            CustomValueType::NuLazyGroupBy(cv) => cv
                .custom_value_partial_cmp(self, engine, other_value)
                .map_err(LabeledError::from),
            CustomValueType::NuWhen(cv) => cv
                .custom_value_partial_cmp(self, engine, other_value)
                .map_err(LabeledError::from),
        }
    }
}

pub trait PolarsPluginCustomValue: CustomValue {
    type PhysicalType: Clone;

    fn id(&self) -> &Uuid;

    fn internal(&self) -> &Option<Self::PhysicalType>;

    fn custom_value_to_base_value(
        &self,
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
    ) -> Result<Value, ShellError>;

    fn custom_value_operation(
        &self,
        _plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        _lhs_span: Span,
        operator: Spanned<Operator>,
        _right: Value,
    ) -> Result<Value, ShellError> {
        Err(ShellError::UnsupportedOperator {
            operator: operator.item,
            span: operator.span,
        })
    }

    fn custom_value_follow_path_int(
        &self,
        _plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        self_span: Span,
        _index: Spanned<usize>,
    ) -> Result<Value, ShellError> {
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: self_span,
        })
    }

    fn custom_value_follow_path_string(
        &self,
        _plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        self_span: Span,
        _column_name: Spanned<String>,
    ) -> Result<Value, ShellError> {
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: self_span,
        })
    }

    fn custom_value_partial_cmp(
        &self,
        _plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        _other_value: Value,
    ) -> Result<Option<Ordering>, ShellError> {
        Ok(None)
    }
}

pub trait CustomValueSupport: Cacheable {
    type CV: PolarsPluginCustomValue<PhysicalType = Self> + CustomValue + 'static;

    fn type_name() -> &'static str;

    fn custom_value(self) -> Self::CV;

    fn base_value(self, span: Span) -> Result<Value, ShellError>;

    fn into_value(self, span: Span) -> Value {
        Value::custom_value(Box::new(self.custom_value()), span)
    }

    fn try_from_custom_value(plugin: &PolarsPlugin, cv: &Self::CV) -> Result<Self, ShellError> {
        if let Some(internal) = cv.internal() {
            Ok(internal.clone())
        } else {
            Self::get_cached(plugin, cv.id())?.ok_or_else(|| ShellError::GenericError {
                error: format!("Dataframe {:?} not found in cache", cv.id()),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
        }
    }

    fn try_from_value(plugin: &PolarsPlugin, value: &Value) -> Result<Self, ShellError> {
        if let Value::CustomValue { val, .. } = value {
            if let Some(cv) = val.as_any().downcast_ref::<Self::CV>() {
                Self::try_from_custom_value(plugin, cv)
            } else {
                Err(ShellError::CantConvert {
                    to_type: Self::type_name().into(),
                    from_type: value.get_type().to_string(),
                    span: value.span(),
                    help: None,
                })
            }
        } else {
            Err(ShellError::CantConvert {
                to_type: Self::type_name().into(),
                from_type: value.get_type().to_string(),
                span: value.span(),
                help: None,
            })
        }
    }

    fn try_from_pipeline(
        plugin: &PolarsPlugin,
        input: PipelineData,
        span: Span,
    ) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(plugin, &value)
    }

    fn can_downcast(value: &Value) -> bool {
        if let Value::CustomValue { val, .. } = value {
            val.as_any().downcast_ref::<Self::CV>().is_some()
        } else {
            false
        }
    }
}
