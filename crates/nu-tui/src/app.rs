//! Pipeline value that carries a TUI definition between `tui` commands.
use crate::widget::{TYPE_NAME, Widget, WidgetKind};
use nu_protocol::engine::Closure;
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{
    CustomValue, IntoPipelineData, ListStream, PipelineData, PipelineMetadata, Record, ShellError,
    Signals, Span, Type, Value,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::{HashMap, HashSet};

/// Metadata key used to ride a [`TuiApp`] on a live stream without collecting it.
pub const TUI_APP_META: &str = "tui_app";

/// A `tui bind` entry: a normalized chord and the hook it runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bind {
    pub chord: String,
    pub closure: Closure,
}

/// Composable TUI definition passed through the pipeline as a custom value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiApp {
    /// Root widgets: chrome, top-level tabs/splits, and bare content.
    pub widgets: Vec<Widget>,
    /// Data collected from the outer pipeline. Widgets without their own
    /// `data` show this.
    pub data: Value,
    /// Path columns from pipeline metadata (`ls` sets `name`). Used for LS_COLORS.
    #[serde(default)]
    pub path_columns: Vec<String>,
    /// Global key bindings from `tui bind`.
    #[serde(default)]
    pub binds: Vec<Bind>,
    /// The outer pipeline is a live stream, so later builders pass it
    /// through untouched.
    #[serde(default)]
    pub live: bool,
    /// The external command feeding the live stream. `tui run` kills it when
    /// the TUI closes while the stream is still open, so the pipeline does
    /// not hang waiting for its exit status.
    #[serde(default)]
    pub live_pid: Option<u32>,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            data: Value::nothing(Span::unknown()),
            path_columns: Vec::new(),
            binds: Vec::new(),
            live: false,
            live_pid: None,
        }
    }

    /// Split the app from the payload. A collected value becomes the app's
    /// data. A stream is collected or kept live by its kind (see
    /// [`crate::stream::collect_input`]): a collected one becomes the data
    /// too (so `(ls | tui table)` works inside a child list), a live one
    /// flows on with the app riding in its metadata.
    pub fn split_input(input: PipelineData) -> Result<(Self, PipelineData), ShellError> {
        match input {
            PipelineData::Empty => Ok((Self::new(), PipelineData::Empty)),
            PipelineData::Value(value, meta) => {
                if let Some(app) = Self::from_value(&value) {
                    return Ok((app.clone(), PipelineData::Empty));
                }
                let mut app = app_from_meta(meta.as_ref());
                // A range is lazy (`1..` never ends), so it is read like a
                // stream.
                if let Value::Range { val, .. } = &value {
                    let span = value.span();
                    let stream = ListStream::new(
                        val.clone().into_range_iter(span, Signals::empty()),
                        span,
                        Signals::empty(),
                    );
                    return Self::absorb_stream(app, PipelineData::ListStream(stream, meta), span);
                }
                if !value.is_nothing() {
                    app.data = value;
                }
                Ok((app, PipelineData::Empty))
            }
            PipelineData::ListStream(stream, meta) => {
                let app = app_from_meta(meta.as_ref());
                let span = stream.span();
                Self::absorb_stream(app, PipelineData::ListStream(stream, meta), span)
            }
            PipelineData::ByteStream(stream, meta) => {
                let app = app_from_meta(meta.as_ref());
                let span = stream.span();
                Self::absorb_stream(app, PipelineData::ByteStream(stream, meta), span)
            }
        }
    }

    fn absorb_stream(
        mut app: Self,
        data: PipelineData,
        span: Span,
    ) -> Result<(Self, PipelineData), ShellError> {
        let meta = data.metadata_ref().cloned();
        if app.live {
            return Ok((app, data.set_metadata(strip_app_meta(meta))));
        }
        match crate::stream::collect_input(data, span) {
            Some(crate::stream::Collected::Done(items)) => {
                app.data = Value::list(items, span);
                Ok((app, PipelineData::Empty))
            }
            Some(crate::stream::Collected::Live { stream, child_pid }) => {
                app.live = true;
                app.live_pid = child_pid;
                Ok((app, PipelineData::ListStream(stream, strip_app_meta(meta))))
            }
            None => Ok((app, PipelineData::Empty)),
        }
    }

    /// Downcast a `tui` custom value.
    pub fn from_value(value: &Value) -> Option<&TuiApp> {
        value
            .as_custom_value()
            .ok()
            .and_then(|custom| custom.as_any().downcast_ref::<TuiApp>())
    }

    /// Emit a custom value when there is no stream; otherwise attach the app
    /// to the stream's metadata so it keeps flowing.
    pub fn emit(self, data: PipelineData, span: Span) -> PipelineData {
        match data {
            PipelineData::Empty => self.into_pipeline_data(span),
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
    ///
    /// Data piped into a child (`(ls | tui table)`) is bound to that child's
    /// root widgets, so each child can show its own rows.
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
                        if let Some(source) = &mut w.source
                            && let Some(from) = &mut source.from
                            && let Some(new) = renamed.get(from)
                        {
                            *from = new.clone();
                        }
                    });
                }
            }
            if !child.data.is_nothing() {
                let mut roots: Vec<&mut Widget> =
                    widgets.iter_mut().filter(|w| w.data.is_none()).collect();
                if let Some(last) = roots.pop() {
                    for root in roots {
                        root.data = Some(child.data.clone());
                    }
                    last.data = Some(child.data);
                }
            }
            out.extend(widgets);
        }
        Ok(out)
    }

    pub fn into_pipeline_data(self, span: Span) -> PipelineData {
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

pub fn string_list(items: &[String], span: Span) -> Value {
    Value::list(
        items
            .iter()
            .map(|s| Value::string(s.clone(), span))
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
    rec.insert("type", Value::string(widget.kind.type_name(), span));
    widget.kind.describe(&mut rec, span);
    if let Some(source) = &widget.source {
        if let Some(from) = &source.from {
            rec.insert("from", Value::string(from.clone(), span));
        }
        rec.insert(
            "has_source_closure",
            Value::bool(source.closure.is_some(), span),
        );
    }
    if widget.data.is_some() {
        rec.insert("has_data", Value::bool(true, span));
    }
    if widget.on_select.is_some() {
        rec.insert("has_on_select", Value::bool(true, span));
    }
    if widget.focus {
        rec.insert("focus", Value::bool(true, span));
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
        TYPE_NAME.to_string()
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
        if !self.binds.is_empty() {
            rec.insert(
                "binds",
                Value::list(
                    self.binds
                        .iter()
                        .map(|b| Value::string(b.chord.clone(), span))
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
    Type::custom(TYPE_NAME)
}
