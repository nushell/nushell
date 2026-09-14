//! Pipeline value that carries a TUI definition between `tui` commands.
use super::widget::{Widget, WidgetKind};
use nu_protocol::{
    CustomValue, IntoPipelineData, PipelineData, PipelineMetadata, Record, ShellError, Span, Type,
    Value,
};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Metadata key used to ride a [`TuiApp`] on a live stream without collecting it.
pub const TUI_APP_META: &str = "tui_app";

/// Composable TUI definition passed through the pipeline as a custom value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiApp {
    pub widgets: Vec<Widget>,
    /// Data captured from a non-TUI pipeline input (tables, lists, records).
    pub data: Value,
    /// Path columns from pipeline metadata (`ls` sets `name`). Used for LS_COLORS.
    #[serde(default)]
    pub path_columns: Vec<String>,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            data: Value::nothing(Span::unknown()),
            path_columns: Vec::new(),
        }
    }

    /// Split widgets from payload. List/byte streams are not collected.
    pub fn split_input(
        input: PipelineData,
        _span: Span,
    ) -> Result<(Self, PipelineData), ShellError> {
        match input {
            PipelineData::Empty => Ok((Self::new(), PipelineData::Empty)),
            PipelineData::Value(value, meta) => {
                if let Ok(custom) = value.as_custom_value()
                    && let Some(app) = custom.as_any().downcast_ref::<TuiApp>()
                {
                    return Ok((app.clone(), PipelineData::Empty));
                }
                let app = app_from_meta(meta.as_ref());
                if value.is_nothing() {
                    Ok((app, PipelineData::Empty))
                } else if is_scalar_payload(&value) && app.widgets.is_empty() {
                    let mut app = app;
                    app.data = value;
                    Ok((app, PipelineData::Empty))
                } else {
                    Ok((app, PipelineData::Value(value, strip_app_meta(meta))))
                }
            }
            PipelineData::ListStream(stream, meta) => {
                let app = app_from_meta(meta.as_ref());
                Ok((app, PipelineData::ListStream(stream, strip_app_meta(meta))))
            }
            PipelineData::ByteStream(stream, meta) => {
                let app = app_from_meta(meta.as_ref());
                Ok((app, PipelineData::ByteStream(stream, strip_app_meta(meta))))
            }
        }
    }

    /// Emit a custom value when there is no payload; otherwise attach the app
    /// to pipeline metadata so streams keep flowing.
    pub fn emit(self, data: PipelineData, span: Span) -> PipelineData {
        match data {
            PipelineData::Empty => self.into_pipeline_data(span),
            PipelineData::Value(value, _) if value.is_nothing() => self.into_pipeline_data(span),
            mut other => {
                let mut meta = other.take_metadata().unwrap_or_default();
                meta.custom
                    .insert(TUI_APP_META, Value::custom(Box::new(self), span));
                other.set_metadata(Some(meta))
            }
        }
    }

    pub fn push(&mut self, widget: Widget) {
        self.widgets.push(widget);
    }

    pub fn next_id(
        &self,
        prefix: &str,
        requested: Option<String>,
        span: Span,
    ) -> Result<String, ShellError> {
        if let Some(id) = requested {
            if self.widgets.iter().any(|w| w.id == id) {
                return Err(ShellError::Generic(
                    nu_protocol::shell_error::generic::GenericError::new(
                        "duplicate tui widget id",
                        format!("a widget with id '{id}' is already in this TUI"),
                        span,
                    ),
                ));
            }
            return Ok(id);
        }
        let mut n = 0usize;
        loop {
            let id = format!("{prefix}-{n}");
            if !self.widgets.iter().any(|w| w.id == id) {
                return Ok(id);
            }
            n += 1;
        }
    }

    pub fn into_pipeline_data(self, span: Span) -> nu_protocol::PipelineData {
        Value::custom(Box::new(self), span).into_pipeline_data()
    }

    pub fn widget(&self, id: &str) -> Option<&Widget> {
        self.widgets.iter().find(|w| w.id == id)
    }

    pub fn widget_kind(&self, id: &str) -> Option<&WidgetKind> {
        self.widget(id).map(|w| &w.kind)
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

fn app_from_meta(meta: Option<&PipelineMetadata>) -> TuiApp {
    let mut app = meta
        .and_then(|m| m.custom.get(TUI_APP_META))
        .and_then(|v| v.as_custom_value().ok())
        .and_then(|custom| custom.as_any().downcast_ref::<TuiApp>().cloned())
        .unwrap_or_default();
    if app.path_columns.is_empty()
        && let Some(m) = meta
        && !m.path_columns.is_empty()
    {
        app.path_columns = m.path_columns.clone();
    }
    app
}

fn strip_app_meta(mut meta: Option<PipelineMetadata>) -> Option<PipelineMetadata> {
    if let Some(m) = meta.as_mut() {
        m.custom.remove(TUI_APP_META);
        if m.custom.is_empty()
            && m.content_type.is_none()
            && m.path_columns.is_empty()
            && matches!(m.data_source, nu_protocol::DataSource::None)
        {
            return None;
        }
    }
    meta
}

fn is_scalar_payload(value: &Value) -> bool {
    matches!(
        value,
        Value::String { .. }
            | Value::Int { .. }
            | Value::Float { .. }
            | Value::Bool { .. }
            | Value::Duration { .. }
            | Value::Date { .. }
            | Value::Filesize { .. }
    )
}

fn widget_to_record(widget: &Widget, span: Span) -> Value {
    let mut rec = Record::new();
    rec.insert("id", Value::string(widget.id.clone(), span));
    rec.insert(
        "type",
        Value::string(widget.kind.type_name().to_string(), span),
    );
    match &widget.kind {
        WidgetKind::Title { text } => {
            rec.insert("text", Value::string(text.clone(), span));
        }
        WidgetKind::Menu { items } => {
            rec.insert(
                "items",
                Value::list(
                    items
                        .iter()
                        .map(|s| Value::string(s.clone(), span))
                        .collect(),
                    span,
                ),
            );
        }
        WidgetKind::Label { text } => {
            rec.insert("text", Value::string(text.clone(), span));
        }
        WidgetKind::TextBox {
            placeholder,
            editable,
            value,
        } => {
            rec.insert("placeholder", Value::string(placeholder.clone(), span));
            rec.insert("editable", Value::bool(*editable, span));
            rec.insert("value", Value::string(value.clone(), span));
        }
        WidgetKind::Table { columns, data } => {
            rec.insert(
                "columns",
                Value::list(
                    columns
                        .iter()
                        .map(|s| Value::string(s.clone(), span))
                        .collect(),
                    span,
                ),
            );
            if let Some(data) = data {
                rec.insert("data", data.clone());
            }
        }
        WidgetKind::Body { title } => {
            rec.insert("title", Value::string(title.clone(), span));
        }
        WidgetKind::Status { text } => {
            rec.insert("text", Value::string(text.clone(), span));
        }
        WidgetKind::Keybindings { data } => {
            if let Some(data) = data {
                rec.insert("data", data.clone());
            }
        }
        WidgetKind::Search {
            placeholder,
            bind,
            target,
        } => {
            rec.insert("placeholder", Value::string(placeholder.clone(), span));
            if let Some(bind) = bind {
                rec.insert("bind", Value::string(bind.clone(), span));
            }
            if let Some(target) = target {
                rec.insert("target", Value::string(target.clone(), span));
            }
        }
        WidgetKind::Splitter { direction, ratio } => {
            rec.insert(
                "direction",
                Value::string(direction.as_str().to_string(), span),
            );
            rec.insert("ratio", Value::int(*ratio as i64, span));
        }
        WidgetKind::Preview {
            column,
            max_bytes,
            transform,
            from,
        } => {
            rec.insert("column", Value::string(column.clone(), span));
            rec.insert("max_bytes", Value::int(*max_bytes as i64, span));
            rec.insert("has_transform", Value::bool(transform.is_some(), span));
            if let Some(from) = from {
                rec.insert("from", Value::string(from.clone(), span));
            }
        }
        WidgetKind::List { data } => {
            if let Some(data) = data {
                rec.insert("data", data.clone());
            }
        }
        WidgetKind::Log { max_lines } => {
            rec.insert("max_lines", Value::int(*max_lines as i64, span));
        }
        WidgetKind::Tree { data, walk, column } => {
            rec.insert("walk", Value::bool(*walk, span));
            rec.insert("column", Value::string(column.clone(), span));
            if let Some(data) = data {
                rec.insert("data", data.clone());
            }
        }
        WidgetKind::Tab { title } => {
            rec.insert("title", Value::string(title.clone(), span));
        }
        WidgetKind::Tabs => {}
    }
    if let Some(place) = &widget.place {
        rec.insert(
            "place",
            Value::record(
                {
                    let mut p = Record::new();
                    p.insert(
                        "rel",
                        Value::string(
                            match place.rel {
                                super::widget::Rel::RightOf => "right-of",
                                super::widget::Rel::LeftOf => "left-of",
                                super::widget::Rel::Above => "above",
                                super::widget::Rel::Below => "below",
                            },
                            span,
                        ),
                    );
                    p.insert("of", Value::string(place.of.clone(), span));
                    p.insert("ratio", Value::int(place.ratio as i64, span));
                    p
                },
                span,
            ),
        );
    }
    Value::record(rec, span)
}

#[typetag::serde]
impl CustomValue for TuiApp {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "tui".to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let mut rec = Record::new();
        rec.insert(
            "widgets",
            Value::list(
                self.widgets
                    .iter()
                    .map(|w| widget_to_record(w, span))
                    .collect(),
                span,
            ),
        );
        rec.insert("data", self.data.clone());
        if !self.path_columns.is_empty() {
            rec.insert(
                "path_columns",
                Value::list(
                    self.path_columns
                        .iter()
                        .map(|c| Value::string(c.clone(), span))
                        .collect(),
                    span,
                ),
            );
        }
        Ok(Value::record(rec, span))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
        optional: bool,
        _casing: nu_protocol::casing::Casing,
    ) -> Result<Value, ShellError> {
        let base = self.to_base_value(self_span)?;
        match base
            .as_record()
            .ok()
            .and_then(|r| r.get(&column_name))
            .cloned()
        {
            Some(v) => Ok(v),
            None if optional => Ok(Value::nothing(path_span)),
            None => Err(ShellError::CantFindColumn {
                col_name: column_name,
                span: Some(path_span),
                src_span: self_span,
            }),
        }
    }
}

/// Shared input/output type for component commands.
pub fn tui_type() -> Type {
    Type::custom("tui")
}
