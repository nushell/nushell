//! Pipeline value that carries a TUI definition between `tui` commands.
use super::widget::{MenuItem, Widget, WidgetKind};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{
    CustomValue, IntoPipelineData, PipelineData, PipelineMetadata, Record, ShellError, Span, Type,
    Value,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::{HashMap, HashSet};

/// Metadata key used to ride a [`TuiApp`] on a live stream without collecting it.
pub const TUI_APP_META: &str = "tui_app";

/// Composable TUI definition passed through the pipeline as a custom value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiApp {
    /// Root widgets: chrome, top-level tabs/splits, and bare content.
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
                if let Some(app) = Self::from_value(&value) {
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

    /// Downcast a `tui` custom value.
    pub fn from_value(value: &Value) -> Option<&TuiApp> {
        value
            .as_custom_value()
            .ok()
            .and_then(|custom| custom.as_any().downcast_ref::<TuiApp>())
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

    /// Every widget in preorder (a container before its children).
    pub fn iter(&self) -> impl Iterator<Item = &Widget> {
        let mut out = Vec::new();
        collect_preorder(&self.widgets, &mut out);
        out.into_iter()
    }

    /// Child-index path from the roots to every widget id, e.g. `[1, 0]`
    /// for the first child of the second root. Prefixes are ancestors.
    pub fn paths(&self) -> HashMap<String, Vec<usize>> {
        let mut out = HashMap::new();
        let mut path = Vec::new();
        collect_paths(&self.widgets, &mut path, &mut out);
        out
    }

    /// Widget at a child-index path. An empty path has no widget.
    pub fn at_path(&self, path: &[usize]) -> Option<&Widget> {
        let (first, rest) = path.split_first()?;
        let mut node = self.widgets.get(*first)?;
        for idx in rest {
            node = node.children.get(*idx)?;
        }
        Some(node)
    }

    /// The id for a new widget: `requested` if given (and unused), else the
    /// first free `{prefix}-{n}`. The bool says whether it was generated.
    pub fn next_id(
        &self,
        prefix: &str,
        requested: Option<String>,
        span: Span,
    ) -> Result<(String, bool), ShellError> {
        if let Some(id) = requested {
            if self.widget(&id).is_some() {
                return Err(duplicate_id(&id, span));
            }
            return Ok((id, false));
        }
        Ok((free_id(prefix, |id| self.widget(id).is_none()), true))
    }

    /// Merge separately built child apps under a container. Children are
    /// built in their own subexpressions and each number ids from zero, so
    /// `[(tui table) (tui table)]` arrives as two `table-0`. Generated ids
    /// that collide with anything already in this tree (or in an earlier
    /// child) are renumbered, and `--from` references inside that child are
    /// rewritten to match. An explicit `--id` that collides is an error.
    pub fn adopt_children(
        &self,
        children: Vec<TuiApp>,
        container_id: &str,
        span: Span,
    ) -> Result<Vec<Widget>, ShellError> {
        let mut taken: HashSet<String> = self.iter().map(|w| w.id.clone()).collect();
        taken.insert(container_id.to_string());
        let mut out = Vec::new();
        for child in children {
            let own: HashSet<String> = child.iter().map(|w| w.id.clone()).collect();
            let mut renamed: HashMap<String, String> = HashMap::new();
            let mut widgets = child.widgets;
            for w in &mut widgets {
                let mut err = None;
                w.for_each_mut(&mut |w| {
                    if err.is_some() {
                        return;
                    }
                    if taken.contains(&w.id) {
                        if !w.auto_id {
                            err = Some(duplicate_id(&w.id, span));
                            return;
                        }
                        let fresh = free_id(w.kind.type_name(), |id| {
                            !taken.contains(id) && !own.contains(id)
                        });
                        renamed.insert(std::mem::replace(&mut w.id, fresh.clone()), fresh);
                    }
                    taken.insert(w.id.clone());
                });
                if let Some(err) = err {
                    return Err(err);
                }
            }
            if !renamed.is_empty() {
                for w in &mut widgets {
                    w.for_each_mut(&mut |w| {
                        if let WidgetKind::Preview {
                            from: Some(from), ..
                        } = &mut w.kind
                            && let Some(new) = renamed.get(from)
                        {
                            *from = new.clone();
                        }
                    });
                }
            }
            out.extend(widgets);
        }
        Ok(out)
    }

    pub fn into_pipeline_data(self, span: Span) -> nu_protocol::PipelineData {
        Value::custom(Box::new(self), span).into_pipeline_data()
    }

    pub fn widget(&self, id: &str) -> Option<&Widget> {
        self.iter().find(|w| w.id == id)
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

fn free_id(prefix: &str, is_free: impl Fn(&str) -> bool) -> String {
    (0usize..)
        .map(|n| format!("{prefix}-{n}"))
        .find(|id| is_free(id))
        .unwrap_or_else(|| format!("{prefix}-0"))
}

fn duplicate_id(id: &str, span: Span) -> ShellError {
    ShellError::Generic(GenericError::new(
        "duplicate tui widget id",
        format!("a widget with id '{id}' is already in this TUI; pass a different --id"),
        span,
    ))
}

fn collect_preorder<'a>(widgets: &'a [Widget], out: &mut Vec<&'a Widget>) {
    for w in widgets {
        out.push(w);
        collect_preorder(&w.children, out);
    }
}

fn collect_paths(widgets: &[Widget], path: &mut Vec<usize>, out: &mut HashMap<String, Vec<usize>>) {
    for (i, w) in widgets.iter().enumerate() {
        path.push(i);
        out.insert(w.id.clone(), path.clone());
        collect_paths(&w.children, path, out);
        path.pop();
    }
}

fn app_from_meta(meta: Option<&PipelineMetadata>) -> TuiApp {
    let mut app = meta
        .and_then(|m| m.custom.get(TUI_APP_META))
        .and_then(TuiApp::from_value)
        .cloned()
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

fn string_list(items: &[String], span: Span) -> Value {
    Value::list(
        items
            .iter()
            .map(|s| Value::string(s.clone(), span))
            .collect(),
        span,
    )
}

fn menu_items_to_value(items: &[MenuItem], span: Span) -> Value {
    Value::list(
        items
            .iter()
            .map(|item| {
                let mut rec = Record::new();
                rec.insert("name", Value::string(item.label.clone(), span));
                if let Some(m) = item.mnemonic {
                    rec.insert("mnemonic", Value::string(m.to_string(), span));
                }
                if !item.items.is_empty() {
                    rec.insert("items", menu_items_to_value(&item.items, span));
                }
                rec.insert("has_action", Value::bool(item.action.is_some(), span));
                Value::record(rec, span)
            })
            .collect(),
        span,
    )
}

/// Record view of one widget's definition. `extend` adds per-widget fields
/// (used by `tui debug` for resolved layout and focus information) and is
/// applied to the widget and, recursively, to its children.
pub fn widget_to_record(
    widget: &Widget,
    span: Span,
    extend: &dyn Fn(&Widget, &mut Record),
) -> Value {
    let mut rec = Record::new();
    rec.insert("id", Value::string(widget.id.clone(), span));
    rec.insert(
        "type",
        Value::string(widget.kind.type_name().to_string(), span),
    );
    match &widget.kind {
        WidgetKind::Label { text, slot } => {
            rec.insert("text", Value::string(text.clone(), span));
            rec.insert("slot", Value::string(slot.as_str(), span));
        }
        WidgetKind::Menu { items } => {
            rec.insert("items", menu_items_to_value(items, span));
        }
        WidgetKind::TextBox { placeholder, value } => {
            rec.insert("placeholder", Value::string(placeholder.clone(), span));
            rec.insert("value", Value::string(value.clone(), span));
        }
        WidgetKind::Table {
            columns,
            capture_keys,
        } => {
            rec.insert("columns", string_list(columns, span));
            rec.insert("capture_keys", Value::bool(*capture_keys, span));
        }
        WidgetKind::Search { placeholder, bind } => {
            rec.insert("placeholder", Value::string(placeholder.clone(), span));
            if let Some(bind) = bind {
                rec.insert("bind", Value::string(bind.clone(), span));
            }
        }
        WidgetKind::Split { direction, ratio } => {
            rec.insert("direction", Value::string(direction.as_str(), span));
            rec.insert("ratio", Value::int(*ratio as i64, span));
        }
        WidgetKind::Preview {
            max_bytes,
            transform,
            from,
        } => {
            rec.insert("max_bytes", Value::int(*max_bytes as i64, span));
            rec.insert("has_transform", Value::bool(transform.is_some(), span));
            if let Some(from) = from {
                rec.insert("from", Value::string(from.clone(), span));
            }
        }
        WidgetKind::Log { max_lines } => {
            rec.insert("max_lines", Value::int(*max_lines as i64, span));
        }
        WidgetKind::Tree { walk, column } => {
            rec.insert("walk", Value::bool(*walk, span));
            rec.insert("column", Value::string(column.clone(), span));
        }
        WidgetKind::Tab { title } => {
            rec.insert("title", Value::string(title.clone(), span));
        }
    }
    extend(widget, &mut rec);
    if !widget.children.is_empty() {
        rec.insert(
            "children",
            Value::list(
                widget
                    .children
                    .iter()
                    .map(|c| widget_to_record(c, span, extend))
                    .collect(),
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
                    .map(|w| widget_to_record(w, span, &|_, _| {}))
                    .collect(),
                span,
            ),
        );
        rec.insert("data", self.data.clone());
        if !self.path_columns.is_empty() {
            rec.insert("path_columns", string_list(&self.path_columns, span));
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
