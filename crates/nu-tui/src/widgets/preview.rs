//! `tui preview`: text for the row highlighted in a table, tree, or select.
use crate::hooks::closure_arity;
use crate::session::Session;
use crate::widget::{Caps, Effect, PreviewState, TuiWidget, WidgetState};
use nu_engine::ClosureEvalOnce;
use nu_protocol::engine::Closure;
use nu_protocol::{DataSource, IntoPipelineData, PipelineMetadata, Record, Span, Value};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Paragraph, Wrap};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewWidget {
    pub max_bytes: usize,
    /// Zero parameters: receives file contents as `$in`. One parameter:
    /// receives the selected row and its output is the pane text.
    pub transform: Option<Closure>,
}

impl PreviewWidget {
    /// Compute the pane for `row` and store it, resetting scroll when the
    /// title changed.
    pub fn refresh(&self, state: &mut WidgetState, row: Option<&Value>, session: &Session) {
        let Some(preview) = state.as_preview_mut() else {
            return;
        };
        let engine = session.engine.as_ref();
        let wants_row = self
            .transform
            .as_ref()
            .zip(engine)
            .is_some_and(|(c, (engine_state, _))| closure_arity(engine_state, c) > 0);
        let file = match (row, wants_row) {
            // One-parameter closure: it is the source, no file is read.
            (Some(row), true) => FilePreview {
                title: row_path(row).unwrap_or_else(|| "preview".into()),
                text: String::new(),
                path: None,
                transformable: true,
            },
            (Some(row), false) => preview_for_row(row, self.max_bytes, &session.cwd),
            (None, _) => FilePreview {
                title: "preview".into(),
                text: String::new(),
                path: None,
                transformable: false,
            },
        };
        let text = match (file.transformable, &self.transform, engine, row) {
            (true, Some(closure), Some((engine_state, stack)), Some(row)) => apply_transform(
                engine_state,
                stack,
                closure.clone(),
                wants_row.then_some(row),
                &file.text,
                file.path.as_deref(),
            ),
            _ => file.text,
        };
        if preview.title != file.title {
            preview.scroll = 0;
        }
        preview.title = file.title;
        preview.text = text;
    }
}

impl TuiWidget for PreviewWidget {
    fn type_name(&self) -> &'static str {
        "preview"
    }

    fn caps(&self) -> Caps {
        Caps::default()
    }

    fn constraint(&self) -> Constraint {
        Constraint::Min(5)
    }

    fn init_state(&self) -> WidgetState {
        WidgetState::Preview(PreviewState::default())
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("max_bytes", Value::int(self.max_bytes as i64, span));
        rec.insert("has_transform", Value::bool(self.transform.is_some(), span));
    }

    fn scroll(
        &self,
        id: &str,
        state: &mut WidgetState,
        delta: i32,
        session: &Session,
    ) -> Vec<Effect> {
        let height = super::inner_rows(session.areas.get(id), 2);
        if let Some(preview) = state.as_preview_mut() {
            let lines = preview.text.lines().count();
            let max_scroll = lines.saturating_sub(height);
            let step = delta.unsigned_abs() as usize;
            preview.scroll = if delta < 0 {
                preview.scroll.saturating_sub(step)
            } else {
                (preview.scroll + step).min(max_scroll)
            };
        }
        Vec::new()
    }

    fn render(
        &self,
        _id: &str,
        state: &WidgetState,
        frame: &mut Frame,
        area: Rect,
        session: &Session,
        _focused: bool,
    ) {
        let theme = &session.theme;
        let preview = state.as_preview().cloned().unwrap_or_default();
        let title = if preview.title.is_empty() {
            "preview"
        } else {
            preview.title.as_str()
        };
        let block = super::framed(title, false, theme);
        let para = if preview.text.is_empty() {
            Paragraph::new("(nothing to preview)")
                .style(theme.muted())
                .block(block)
        } else {
            Paragraph::new(super::ansi_to_text(&preview.text, theme))
                .style(theme.text())
                .wrap(Wrap { trim: false })
                .scroll((preview.scroll.min(u16::MAX as usize) as u16, 0))
                .block(block)
        };
        frame.render_widget(para, area);
    }

    fn value(&self, _id: &str, state: &WidgetState, _session: &Session) -> Value {
        let span = Span::unknown();
        let preview = state.as_preview().cloned().unwrap_or_default();
        let mut rec = Record::new();
        rec.insert("title", Value::string(preview.title, span));
        rec.insert("text", Value::string(preview.text, span));
        Value::record(rec, span)
    }

    fn debug(&self, id: &str, _state: &WidgetState, session: &Session, rec: &mut Record) {
        let span = Span::unknown();
        rec.insert(
            "source",
            match session.source_id(id) {
                Some(src) => Value::string(src, span),
                None => Value::nothing(span),
            },
        );
    }
}

struct FilePreview {
    title: String,
    text: String,
    path: Option<PathBuf>,
    /// Whether a transform closure should run on `text`.
    transformable: bool,
}

/// Read the file named by the row's `name` column (or the row itself when it
/// is a string).
fn preview_for_row(row: &Value, max_bytes: usize, cwd: &Path) -> FilePreview {
    let Some(name) = row_path(row) else {
        return FilePreview {
            title: "preview".into(),
            text: "(no path on this row)".into(),
            path: None,
            transformable: false,
        };
    };
    let path = {
        let p = Path::new(&name);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        }
    };
    let title = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name.as_str())
        .to_string();

    let kind = row_type(row);
    if kind.as_deref() == Some("dir") || kind.as_deref() == Some("directory") {
        return FilePreview {
            title,
            text: format!("{name}/\n(directory)"),
            path: Some(path),
            transformable: false,
        };
    }

    match std::fs::metadata(&path) {
        Ok(meta) if meta.is_dir() => FilePreview {
            title,
            text: format!("{name}/\n(directory)"),
            path: Some(path),
            transformable: false,
        },
        Ok(meta) if meta.is_file() => {
            let (text, transformable) = read_file_preview(&path, max_bytes, meta.len());
            FilePreview {
                title,
                text,
                path: Some(path),
                transformable,
            }
        }
        Ok(_) => FilePreview {
            title,
            text: format!("{name}\n(not a regular file)"),
            path: Some(path),
            transformable: false,
        },
        Err(err) => FilePreview {
            title,
            text: if kind.as_deref() == Some("file") {
                format!("cannot read {name}: {err}")
            } else {
                format!("{name}\n({err})")
            },
            path: Some(path),
            transformable: false,
        },
    }
}

/// Run the preview closure. With `row`, the closure is the source and `$in`
/// is empty; otherwise `$in` is `text` with the file's `content_type`.
fn apply_transform(
    engine_state: &nu_protocol::engine::EngineState,
    stack: &nu_protocol::engine::Stack,
    closure: Closure,
    row: Option<&Value>,
    text: &str,
    path: Option<&Path>,
) -> String {
    let input = match row {
        Some(_) => nu_protocol::PipelineData::empty(),
        None => {
            let metadata = PipelineMetadata {
                data_source: path
                    .map(|p| DataSource::FilePath(p.to_path_buf()))
                    .unwrap_or_default(),
                content_type: path.and_then(preview_content_type),
                ..Default::default()
            };
            Value::string(text, Span::unknown()).into_pipeline_data_with_metadata(Some(metadata))
        }
    };

    let mut eval = ClosureEvalOnce::new(engine_state, stack, closure);
    if let Some(row) = row {
        eval = match eval.add_arg(row.clone()) {
            Ok(eval) => eval,
            Err(err) => return format!("preview error: {err}"),
        };
    }

    match eval
        .run_with_input(input)
        .and_then(|data| data.into_value(Span::unknown()))
    {
        Ok(Value::String { val, .. }) => val,
        Ok(Value::Binary { val, .. }) => String::from_utf8_lossy(&val).into_owned(),
        Ok(Value::Nothing { .. }) => String::new(),
        Ok(other) => other.to_expanded_string("\n", engine_state.get_config()),
        Err(err) => format!("preview error: {err}"),
    }
}

fn preview_content_type(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "yaml" | "yml" => Some("application/yaml".into()),
        "nu" => Some("application/x-nuscript".into()),
        "json" | "jsonl" | "ndjson" => Some("application/json".into()),
        "nuon" => Some("application/x-nuon".into()),
        other => mime_guess::from_ext(other).first().map(|m| m.to_string()),
    }
}

/// The path a row names: its `name` column, or the row itself when it is a
/// string.
pub fn row_path(row: &Value) -> Option<String> {
    match row {
        Value::Record { val, .. } => val.get("name").and_then(|v| match v {
            Value::String { val, .. } | Value::Glob { val, .. } => Some(val.clone()),
            other => other.as_str().ok().map(str::to_string),
        }),
        Value::String { val, .. } => Some(val.clone()),
        _ => None,
    }
}

fn row_type(row: &Value) -> Option<String> {
    row.as_record()
        .ok()
        .and_then(|r| r.get("type"))
        .and_then(|v| v.as_str().ok())
        .map(|s| s.to_ascii_lowercase())
}

fn read_file_preview(path: &Path, max_bytes: usize, file_len: u64) -> (String, bool) {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(err) => return (format!("cannot open: {err}"), false),
    };
    let mut buf = vec![0u8; max_bytes];
    let n = match std::io::Read::read(&mut file, &mut buf) {
        Ok(n) => n,
        Err(err) => return (format!("cannot read: {err}"), false),
    };
    buf.truncate(n);
    if buf.contains(&0) {
        return (format!("(binary file, {file_len} bytes)"), false);
    }
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    if file_len as usize > n {
        let _ = write!(text, "\n\n… truncated, showing {n} of {file_len} bytes");
    }
    (text, true)
}
