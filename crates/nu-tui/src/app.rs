//! Pipeline value that carries a TUI definition between `tui` commands.
use crate::stream::Feed;
use crate::widget::{TYPE_NAME, Widget, WidgetKind};
use nu_protocol::engine::Closure;
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{
    CustomValue, IntoPipelineData, IntoValue, ListStream, PipelineData, Record, ShellError,
    Signals, Span, Type, Value,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::{HashMap, HashSet};

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
    /// The value piped into the first builder, plus the rows of `stream`
    /// read so far. Widgets without their own `data` show this.
    pub data: Value,
    /// A stream from the outer pipeline, unread until the TUI runs; its
    /// rows are appended to `data` as they arrive.
    #[serde(skip)]
    pub stream: Option<Feed>,
    /// Path columns from pipeline metadata (`ls` sets `name`). Used for LS_COLORS.
    #[serde(default)]
    pub path_columns: Vec<String>,
    /// Global key bindings from `tui bind`.
    #[serde(default)]
    pub binds: Vec<Bind>,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            data: Value::nothing(Span::unknown()),
            stream: None,
            path_columns: Vec::new(),
            binds: Vec::new(),
        }
    }

    /// The `tui` value a builder extends: the one piped in, or a new one
    /// holding the piped data. A collected value becomes the data; a stream
    /// (or a range, which is lazy: `1..` never ends) is kept unread, so a
    /// builder never waits on a producer. The TUI reads it when it runs.
    pub fn from_input(input: PipelineData) -> Self {
        let mut app = Self::new();
        if let Some(meta) = input.metadata_ref() {
            // `ls` marks its path columns, for LS_COLORS.
            app.path_columns = meta.path_columns.clone();
        }
        match input {
            PipelineData::Empty => {}
            PipelineData::Value(value, _) => {
                if let Some(piped) = Self::from_value(&value) {
                    return piped.clone();
                }
                if let Value::Range { val, .. } = &value {
                    let span = value.span();
                    let stream = ListStream::new(
                        val.clone().into_range_iter(span, Signals::empty()),
                        span,
                        Signals::empty(),
                    );
                    app.stream = Feed::from_pipeline(PipelineData::list_stream(stream, None), span);
                } else if !value.is_nothing() {
                    app.data = value;
                }
            }
            stream => {
                let span = stream.span().unwrap_or(Span::unknown());
                app.stream = Feed::from_pipeline(stream, span);
            }
        }
        app
    }

    /// Downcast a `tui` custom value.
    pub fn from_value(value: &Value) -> Option<&TuiApp> {
        value
            .as_custom_value()
            .ok()
            .and_then(|custom| custom.as_any().downcast_ref::<TuiApp>())
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

    /// Mutable [`TuiApp::at_path`].
    pub fn at_path_mut(&mut self, path: &[usize]) -> Option<&mut Widget> {
        let (first, rest) = path.split_first()?;
        let mut node = self.widgets.get_mut(*first)?;
        for idx in rest {
            node = node.children.get_mut(*idx)?;
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
    /// root widgets, so each child can show its own rows. A stream stays
    /// unread: the roots share it and start with an empty list, so they
    /// never show the outer data while they wait for their first row. Only
    /// roots without their own `--data` take piped data; a child with none
    /// is an error.
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
            let piped = child.stream.is_some() || !child.data.is_nothing();
            let mut roots: Vec<&mut Widget> =
                widgets.iter_mut().filter(|w| w.data.is_none()).collect();
            if piped && roots.is_empty() {
                // Dropping it silently would also leave an external
                // producer running with nobody reading it.
                return Err(ShellError::Generic(GenericError::new(
                    "piped data has no widget to show it",
                    "no widget in this child can show the data piped into it: each has its own --data, or there are none",
                    span,
                )));
            }
            if let Some(feed) = child.stream {
                for root in roots {
                    root.data = Some(Value::list(Vec::new(), span));
                    root.stream = Some(feed.clone());
                }
            } else if !child.data.is_nothing()
                && let Some(last) = roots.pop()
            {
                for root in roots {
                    root.data = Some(child.data.clone());
                }
                last.data = Some(child.data);
            }
            out.extend(widgets);
        }
        Ok(out)
    }

    pub fn into_pipeline_data(self, span: Span) -> PipelineData {
        Value::custom(Box::new(self), span).into_pipeline_data()
    }

    /// The widget with `id`, searched in preorder without collecting the
    /// tree: renders look up every widget each frame.
    pub fn widget(&self, id: &str) -> Option<&Widget> {
        find_widget(&self.widgets, id)
    }

    /// Rows a stream keeps before the oldest are dropped (see
    /// [`crate::stream::trim_front`]): 100k, or more when a log keeps more
    /// lines (`--max-lines`). A finite stream up to this size is shown in
    /// full.
    pub fn stream_row_cap(&self) -> usize {
        const DEFAULT: usize = 100_000;
        self.iter()
            .filter_map(|w| match &w.kind {
                WidgetKind::Log(log) => Some(log.max_lines),
                _ => None,
            })
            .fold(DEFAULT, usize::max)
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

fn find_widget<'a>(widgets: &'a [Widget], id: &str) -> Option<&'a Widget> {
    widgets.iter().find_map(|w| {
        if w.id == id {
            Some(w)
        } else {
            find_widget(&w.children, id)
        }
    })
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
    if widget.stream.as_ref().is_some_and(Feed::has_rows) {
        rec.insert("has_stream", Value::bool(true, span));
    }
    if widget.on_select.is_some() {
        rec.insert("has_on_select", Value::bool(true, span));
    }
    if widget.focus {
        rec.insert("focus", Value::bool(true, span));
    }
    if let Some(title) = &widget.title {
        rec.insert("title", Value::string(title.clone(), span));
    }
    if let Some(border) = widget.border {
        rec.insert("border", border.into_value(span));
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
        if self.stream.as_ref().is_some_and(Feed::has_rows) {
            // Its rows are read when the TUI runs.
            rec.insert("stream", Value::bool(true, span));
        }
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
