//! `tui preview`: text or an image for the row highlighted in a table,
//! tree, or select. `tui image` is the same widget with a fixed source.
use crate::hooks::closure_arity;
use crate::image::{self, ImageOptions, Images, MAX_IMAGE_BYTES, View};
use crate::keys::KeyPress;
use crate::session::Session;
use crate::widget::{Caps, Effect, PreviewState, TuiWidget, WidgetState};
use ::image::DynamicImage;
use nu_engine::ClosureEvalOnce;
use nu_protocol::engine::{Closure, CurrentJob, EngineState, Mailbox, Stack, ThreadJob};
use nu_protocol::{
    DataSource, IntoPipelineData, JobId, PipelineMetadata, Record, Signals, Span, Value, record,
};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Paragraph;
use ratatui_image::Image;
use ratatui_image::protocol::Protocol;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, Weak};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreviewWidget {
    pub max_bytes: usize,
    /// Zero parameters: receives file contents as `$in`. One parameter:
    /// receives the selected row and its output is the pane text.
    pub transform: Option<Closure>,
    /// `tui image`: the path or bytes shown, instead of a followed row.
    #[serde(default)]
    pub fixed: Option<Value>,
    #[serde(default)]
    pub image: ImageOptions,
    /// `--threads`: read the file or run the closure on a thread, so a slow
    /// source does not hold up the rest of the TUI.
    #[serde(default)]
    pub threads: bool,
    /// `--on-change`: a hook run when the file on show changes, with the
    /// state record (`values.<id>.row` is the new row). What it returns is
    /// the pane's fresh `views.<id>` part of the app context, which the
    /// closure sees laid over the shared keys as `$ctx`. Without it the
    /// part is dropped.
    #[serde(default)]
    pub on_change: Option<Closure>,
}

/// An image on show: decoded once per source, re-encoded whenever the pane
/// or the terminal's protocol needs it.
pub struct PreviewImage {
    /// What `source` was decoded from: the file path, or `<binary>` for
    /// bytes a closure returned. A resize re-encodes without decoding again.
    pub key: String,
    pub source: Arc<DynamicImage>,
    pub format: String,
    pub protocol: Protocol,
}

impl Drop for PreviewImage {
    /// An image dropped before it was drawn (a zoom step the next one
    /// overtook) may hold a shared memory object the terminal never read.
    fn drop(&mut self) {
        image::release_untransmitted(&self.protocol);
    }
}

impl std::fmt::Debug for PreviewImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewImage")
            .field("key", &self.key)
            .field("format", &self.format)
            .field("size", &self.protocol.size())
            .finish_non_exhaustive()
    }
}

/// The image a pane shows. Decoding and encoding run on a thread so the
/// list keeps moving over large files; the pane draws the result once
/// [`Session::poll_images`] collects it.
#[derive(Debug, Clone)]
pub enum ImageState {
    /// A job is still running. `shown` is the image drawn meanwhile: the
    /// same source re-encoded for a new zoom, pan, or pane, which stays up
    /// so it does not flicker. A new source shows the pane text instead.
    Loading {
        key: String,
        shown: Option<Arc<PreviewImage>>,
        job: ImageJob,
    },
    Ready(Arc<PreviewImage>),
}

impl ImageState {
    /// The image to draw now, if any.
    pub fn shown(&self) -> Option<&Arc<PreviewImage>> {
        match self {
            Self::Loading { shown, .. } => shown.as_ref(),
            Self::Ready(image) => Some(image),
        }
    }

    fn key(&self) -> &str {
        match self {
            Self::Loading { key, .. } => key,
            Self::Ready(image) => &image.key,
        }
    }
}

/// A running preview job. Key handlers work on a copy of the widget state,
/// so a job may have several handles; once the last is dropped the job is
/// cancelled: the thread stops at its next check, an external it started
/// is killed, and its result goes nowhere.
#[derive(Debug, Clone)]
pub struct ImageJob {
    rx: Arc<Mutex<Receiver<Loaded>>>,
    /// The thread holds a `Weak` to this and stops once it is gone.
    _token: Arc<Cancel>,
}

/// Dropped with the last handle on a job. A closure running on the thread
/// has its own engine job, so dropping this interrupts it and kills the
/// externals it spawned, the way `job kill` does.
#[derive(Debug, Default)]
struct Cancel {
    engine_job: Option<ThreadJob>,
}

impl Drop for Cancel {
    fn drop(&mut self) {
        if let Some(job) = &self.engine_job {
            // Already dead when the job finished on its own.
            let _ = job.kill();
        }
    }
}

/// What a job produces: the image for the pane (unless it could not be
/// decoded or drawn), the pane text (the image summary, or what went
/// wrong), and the view with its pan clamped to the encoded image.
#[derive(Debug)]
pub struct Loaded {
    image: Option<PreviewImage>,
    text: String,
    view: View,
}

/// A job's input: the row to read or run the closure for (`--threads`),
/// bytes to decode, or an image decoded by an earlier job.
enum Input {
    Source(Box<Source>),
    Bytes(Vec<u8>),
    Decoded(Arc<DynamicImage>, String),
}

/// What [`source_body`] needs, owned so a thread can run it. The engine is
/// a clone with its own signals and job (see [`detach_engine`]).
struct Source {
    row: Option<Value>,
    context: Value,
    engine: Option<(EngineState, Stack)>,
    transform: Option<Closure>,
    wants_row: bool,
    max_bytes: usize,
    cwd: PathBuf,
}

impl Source {
    fn body(&self) -> Body {
        source_body(
            self.row.as_ref(),
            &self.context,
            self.wants_row,
            self.transform.as_ref(),
            self.engine.as_ref().map(|(e, s)| (e, s)),
            self.max_bytes,
            &self.cwd,
        )
    }
}

/// Everything a job needs, cloned from the session so the thread owns it.
struct Job {
    /// Identifies the decoded image; empty until a `Source` has been read.
    key: String,
    input: Input,
    /// The image on show, reused undecoded when the source gives the same
    /// key again (a resize, a closure that returned the same bytes).
    shown: Option<Arc<PreviewImage>>,
    images: Option<Images>,
    options: ImageOptions,
    pane: (u16, u16),
    view: View,
    kitty_id: u32,
}

impl Job {
    /// Read the source if there is one, then decode and encode, unless
    /// `token` says nobody wants the result.
    fn run(mut self, token: &Weak<Cancel>) -> Option<Loaded> {
        if let Input::Source(source) = &self.input {
            let body = source.body();
            if token.strong_count() == 0 {
                return None;
            }
            match body {
                Body::Text { text, .. } => {
                    return Some(Loaded {
                        image: None,
                        text,
                        view: self.view,
                    });
                }
                Body::Image(bytes) => {
                    self.key = image_key(source.row.as_ref(), bytes.len());
                    self.input = match self.shown.take().filter(|i| i.key == self.key) {
                        Some(same) => Input::Decoded(same.source.clone(), same.format.clone()),
                        None => Input::Bytes(bytes),
                    };
                }
            }
        }
        let failed = |text: String| Loaded {
            image: None,
            text,
            view: self.view,
        };
        let bytes = match &self.input {
            Input::Bytes(bytes) => bytes.len(),
            Input::Decoded(..) | Input::Source(_) => 0,
        };
        let (source, format) = match self.input {
            Input::Source(_) => return None,
            Input::Decoded(source, format) => (source, format),
            Input::Bytes(bytes) => match image::decode(&bytes) {
                Some(Ok((image, format))) => (Arc::new(image), format),
                Some(Err(err)) => {
                    return Some(failed(format!("(cannot decode the image: {err})")));
                }
                None => return Some(failed("(not an image)".to_string())),
            },
        };
        if token.strong_count() == 0 {
            return None;
        }
        let summary = format!("image {}x{} {format}", source.width(), source.height());
        let Some(images) = &self.images else {
            return Some(failed(format!("({summary}, {bytes} bytes)")));
        };
        let mut view = self.view;
        let encoded = images.encode(&source, &self.options, self.pane, &mut view, self.kitty_id);
        // Cancelled while encoding: the result is dropped, which releases
        // whatever the transmission holds outside the process.
        if token.strong_count() == 0 {
            return None;
        }
        Some(match encoded {
            Ok(protocol) => Loaded {
                image: Some(PreviewImage {
                    key: self.key,
                    source,
                    format,
                    protocol,
                }),
                text: summary,
                view,
            },
            Err(err) => failed(format!("{summary}\n(cannot show it: {err})")),
        })
    }
}

/// What a row gives the pane before any transform: text, or an image's
/// bytes.
enum Body {
    Text {
        text: String,
        /// Whether a transform closure should run on `text`.
        transformable: bool,
    },
    Image(Vec<u8>),
}

impl PreviewWidget {
    /// Compute the pane for `row` and store it, resetting scroll when the
    /// title changed.
    /// `stack` carries `TUI_WIDTH` / `TUI_HEIGHT` for the closure (see
    /// [`Session::closure_stack`]).
    /// `context` is what the closure sees as `$ctx`; the session has run
    /// `--on-change` for a new file already (see [`Self::file_changed`]).
    pub fn refresh(
        &self,
        id: &str,
        state: &mut WidgetState,
        row: Option<&Value>,
        session: &Session,
        stack: Option<&nu_protocol::engine::Stack>,
        context: &Value,
    ) {
        let Some(preview) = state.as_preview_mut() else {
            return;
        };
        let row = self.shown_row(row);
        let engine = session.engine.as_ref().zip(stack);
        // A transform closure whose block is unavailable (for example one parsed
        // after this job was spawned) fails for every row, so report it once
        // instead of falling through to the file preview.
        let wants_row = match self.transform.as_ref().zip(engine) {
            Some((c, ((engine_state, _), _))) => match closure_arity(engine_state, c) {
                Ok(arity) => arity > 0,
                Err(err) => {
                    preview.text = format!("preview error: {err}");
                    preview.image = None;
                    return;
                }
            },
            None => false,
        };
        let title = title_of(row);
        let same_file = preview.title == title;
        if !same_file {
            preview.scroll = 0;
            preview.view.reset();
        }
        preview.title = title;
        if self.threads && row.is_some() {
            // The same file again (a resize) keeps its image up while the
            // source is read again; a new one shows the pane text.
            let shown = same_file
                .then(|| preview.image.as_ref().and_then(ImageState::shown).cloned())
                .flatten();
            let source = Source {
                row: row.cloned(),
                context: context.clone(),
                engine: engine
                    .map(|((engine_state, _), stack)| (engine_state.clone(), stack.clone())),
                transform: self.transform.clone(),
                wants_row,
                max_bytes: self.max_bytes,
                cwd: session.cwd.clone(),
            };
            preview.text = "(loading...)".into();
            self.start_job(
                id,
                preview,
                String::new(),
                Input::Source(Box::new(source)),
                shown,
                session,
            );
            return;
        }
        let body = source_body(
            row,
            context,
            wants_row,
            self.transform.as_ref(),
            engine.map(|((engine_state, _), stack)| (engine_state, stack)),
            self.max_bytes,
            &session.cwd,
        );
        match body {
            Body::Text { text, .. } => {
                preview.text = text;
                preview.image = None;
            }
            Body::Image(bytes) => {
                let key = image_key(row, bytes.len());
                self.show_image(id, preview, key, bytes, session)
            }
        }
    }

    /// The row the pane shows for the source's `row`: `--from` a fixed
    /// row, else that one.
    pub fn shown_row<'a>(&'a self, row: Option<&'a Value>) -> Option<&'a Value> {
        self.fixed.as_ref().or(row)
    }

    /// Whether `row` (from [`Self::shown_row`]) names another file than
    /// the one on show: when `--on-change` runs and the pane's context,
    /// scroll, and zoom start over.
    pub fn file_changed(&self, state: &WidgetState, row: Option<&Value>) -> bool {
        state
            .as_preview()
            .is_some_and(|preview| preview.title != title_of(row))
    }

    /// Show the image in `bytes`: decode it, unless it is the source already
    /// on show (then only the pane changed), and encode it for the pane.
    fn show_image(
        &self,
        id: &str,
        preview: &mut PreviewState,
        key: String,
        bytes: Vec<u8>,
        session: &Session,
    ) {
        let shown = preview
            .image
            .as_ref()
            .filter(|state| state.key() == key)
            .and_then(|state| state.shown().cloned());
        let input = match &shown {
            Some(image) => Input::Decoded(image.source.clone(), image.format.clone()),
            None => Input::Bytes(bytes),
        };
        self.start_job(id, preview, key, input, shown, session);
    }

    /// Re-encode the image on show after its view changed; it stays up
    /// until the new encoding arrives.
    fn re_encode(&self, id: &str, preview: &mut PreviewState, session: &Session) {
        let Some(shown) = preview.image.as_ref().and_then(ImageState::shown).cloned() else {
            return;
        };
        let input = Input::Decoded(shown.source.clone(), shown.format.clone());
        self.start_job(id, preview, shown.key.clone(), input, Some(shown), session);
    }

    /// Start decoding and encoding on a thread, cancelling any job the pane
    /// had running. With nothing to keep showing, the pane says it is
    /// loading until the result arrives.
    fn start_job(
        &self,
        id: &str,
        preview: &mut PreviewState,
        key: String,
        mut input: Input,
        shown: Option<Arc<PreviewImage>>,
        session: &Session,
    ) {
        let mut cancel = Cancel::default();
        if let Input::Source(source) = &mut input
            && let Some((engine_state, _)) = &mut source.engine
        {
            cancel.engine_job = Some(detach_engine(engine_state));
        }
        let job = Job {
            key: key.clone(),
            input,
            shown: shown.clone(),
            images: session.images.clone(),
            options: self.image.clone(),
            pane: session.inner_size(id),
            view: preview.view,
            kitty_id: preview.kitty_id.unwrap_or(1),
        };
        let token = Arc::new(cancel);
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name(format!("tui preview {id}"))
            .spawn({
                let token = Arc::downgrade(&token);
                move || {
                    if let Some(loaded) = job.run(&token) {
                        // The receiver is gone when the job was cancelled.
                        let _ = tx.send(loaded);
                    }
                }
            });
        if let Err(err) = spawned {
            preview.text = format!("(cannot start the preview: {err})");
            preview.image = None;
            return;
        }
        if shown.is_none() && !key.is_empty() {
            preview.text = "(loading the image...)".into();
        }
        preview.image = Some(ImageState::Loading {
            key,
            shown,
            job: ImageJob {
                rx: Arc::new(Mutex::new(rx)),
                _token: token,
            },
        });
    }

    /// Scroll the pane text by `delta` lines, within what is there.
    fn scroll_text(&self, id: &str, preview: &mut PreviewState, delta: i64, session: &Session) {
        let height = super::inner_rows(session.areas.get(id), 2);
        // Keys stop at the top; only the `scroll` action counts from the end.
        preview.scroll_to((preview.scroll as i64 + delta).max(0), height);
    }
}

impl PreviewState {
    /// Scroll the text to `line`, within what is there; a negative line
    /// counts from the end, so -1 is the last page.
    pub fn scroll_to(&mut self, line: i64, height: usize) {
        let max_scroll = self.text.lines().count().saturating_sub(height) as i64;
        let line = if line < 0 {
            max_scroll + 1 + line
        } else {
            line
        };
        self.scroll = line.clamp(0, max_scroll) as usize;
    }

    /// Take the result of a finished job, if any. `wait` blocks for it,
    /// which keeps a headless run deterministic.
    pub fn poll_image(&mut self, wait: bool) {
        let Some(ImageState::Loading { job, .. }) = &self.image else {
            return;
        };
        let Ok(rx) = job.rx.lock() else {
            return;
        };
        let loaded = if wait {
            rx.recv().ok()
        } else {
            match rx.try_recv() {
                Ok(loaded) => Some(loaded),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => None,
            }
        };
        drop(rx);
        match loaded {
            Some(loaded) => {
                self.text = loaded.text;
                self.view = loaded.view;
                self.image = loaded.image.map(|image| ImageState::Ready(Arc::new(image)));
            }
            // The thread died without a result.
            None => {
                self.text = "(the preview could not be loaded)".into();
                self.image = None;
            }
        }
    }
}

impl TuiWidget for PreviewWidget {
    fn type_name(&self) -> &'static str {
        if self.fixed.is_some() {
            "image"
        } else {
            "preview"
        }
    }

    fn caps(&self) -> Caps {
        Caps {
            focusable: true,
            ..Caps::default()
        }
    }

    fn constraint(&self) -> Constraint {
        Constraint::Min(5)
    }

    fn init_state(&self) -> WidgetState {
        WidgetState::Preview(PreviewState::default())
    }

    fn hint(&self, id: &str, state: &WidgetState, session: &Session) -> Option<String> {
        let preview = state.as_preview()?;
        if let Ok(context) = session.view_context(id).as_record()
            && let Some(Value::String { val, .. }) = context.get("hint")
        {
            return Some(val.clone());
        }
        Some(if preview.image.is_some() {
            format!(
                "+/-:zoom  arrows:pan  0:reset  i:{}",
                preview.view.filter.toggled().name()
            )
        } else {
            "arrows/pgup/pgdn:scroll".into()
        })
    }

    /// Focused, the pane takes the keys a list would: text scrolls by line
    /// or page; an image zooms with `+`/`-`, pans with the arrows or
    /// `hjkl`, `0` shows it as first drawn, and `i` switches the zoom
    /// between smooth and pixel-for-pixel resampling.
    fn handle_key(
        &self,
        id: &str,
        state: &mut WidgetState,
        key: &KeyPress,
        session: &Session,
    ) -> Option<Vec<Effect>> {
        let preview = state.as_preview_mut()?;
        let chord = key.chord.as_str();
        if preview.image.is_none() {
            let height = super::inner_rows(session.areas.get(id), 2) as i64;
            let delta = match chord {
                "up" | "k" => -1,
                "down" | "j" => 1,
                "pageup" => -height,
                "pagedown" => height,
                "home" => i64::MIN / 2,
                "end" => i64::MAX / 2,
                _ => return None,
            };
            self.scroll_text(id, preview, delta, session);
            return Some(Vec::new());
        }
        // A quarter of the pane per pan step, in the zoomed image's pixels.
        let font = session.images.as_ref()?.font_size();
        let (width, height) = session.inner_size(id);
        let step_x = i64::from(width) * i64::from(font.width) / 4;
        let step_y = i64::from(height) * i64::from(font.height) / 4;
        match chord {
            "+" | "=" | "shift++" | "shift+=" => preview.view.zoom_by(View::ZOOM_STEP),
            "-" | "shift+-" | "shift+_" | "_" => preview.view.zoom_by(1.0 / View::ZOOM_STEP),
            "0" | "home" => preview.view.reset(),
            "i" => preview.view.filter = preview.view.filter.toggled(),
            "left" | "h" => preview.view.pan_by(-step_x, 0),
            "right" | "l" => preview.view.pan_by(step_x, 0),
            "up" | "k" => preview.view.pan_by(0, -step_y),
            "down" | "j" => preview.view.pan_by(0, step_y),
            _ => return None,
        }
        self.re_encode(id, preview, session);
        Some(Vec::new())
    }

    fn click(
        &self,
        id: &str,
        _state: &mut WidgetState,
        _area: Rect,
        _x: u16,
        _y: u16,
        _session: &Session,
    ) -> Vec<Effect> {
        vec![Effect::Focus(id.to_string())]
    }

    fn describe(&self, rec: &mut Record, span: Span) {
        rec.insert("max_bytes", Value::int(self.max_bytes as i64, span));
        rec.insert("has_transform", Value::bool(self.transform.is_some(), span));
        rec.insert("image_fit", Value::string(self.image.fit.name(), span));
        rec.insert(
            "image_id",
            match self.image.kitty_id {
                Some(id) => Value::int(i64::from(id), span),
                None => Value::nothing(span),
            },
        );
    }

    fn scroll(
        &self,
        id: &str,
        state: &mut WidgetState,
        delta: i32,
        session: &Session,
    ) -> Vec<Effect> {
        if let Some(preview) = state.as_preview_mut()
            && preview.image.is_none()
        {
            self.scroll_text(id, preview, i64::from(delta), session);
        }
        Vec::new()
    }

    fn render(
        &self,
        id: &str,
        state: &WidgetState,
        frame: &mut Frame,
        area: Rect,
        session: &Session,
        focused: bool,
    ) {
        let theme = &session.theme;
        let empty = PreviewState::default();
        let preview = state.as_preview().unwrap_or(&empty);
        // The previewed file's name is the title; a `--title` goes in front
        // of it. An empty file name means nothing is being previewed.
        let title = match (super::custom_title(session, id, ""), preview.title.as_str()) {
            ("", "") => self.type_name().to_string(),
            ("", file) => file.to_string(),
            (name, "") => name.to_string(),
            (name, file) => format!("{name} ({file})"),
        };
        let block = super::framed(&title, focused, session, id);
        if let Some(image) = preview.image.as_ref().and_then(ImageState::shown) {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            // The protocol was sized for this pane; clipping only matters
            // in the frame between a resize and the refresh it triggers.
            frame.render_widget(Image::new(&image.protocol).allow_clipping(true), inner);
            return;
        }
        if preview.text.is_empty() {
            let note = Paragraph::new("(nothing to preview)")
                .style(theme.muted())
                .block(block);
            frame.render_widget(note, area);
            return;
        }
        // Drawn row by row rather than as a `Paragraph`, so each row's
        // hyperlinks can be put into its cells once it is in the buffer.
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let buf = frame.buffer_mut();
        buf.set_style(inner, theme.text());
        let mut rows = inner.rows();
        let mut skip = preview.scroll;
        for raw in preview.text.split('\n') {
            let (line, links) = super::linked_line(raw, theme);
            let mut first = 0;
            for wrapped in super::wrap_line(&line, inner.width as usize) {
                if skip > 0 {
                    skip -= 1;
                } else {
                    let Some(row) = rows.next() else { return };
                    buf.set_line(row.x, row.y, &wrapped, row.width);
                    super::hyperlink_cells(buf, row, first, &links);
                }
                first += wrapped
                    .spans
                    .iter()
                    .map(|s| super::columns(&s.content))
                    .sum::<usize>();
            }
        }
    }

    fn value(&self, id: &str, state: &WidgetState, session: &Session) -> Value {
        let span = Span::unknown();
        let empty = PreviewState::default();
        let preview = state.as_preview().unwrap_or(&empty);
        let shown = preview.image.as_ref().and_then(ImageState::shown);
        let image = match (shown, &session.images) {
            (Some(image), Some(images)) => Value::record(
                record! {
                    "width" => Value::int(i64::from(image.source.width()), span),
                    "height" => Value::int(i64::from(image.source.height()), span),
                    "format" => Value::string(&image.format, span),
                    "protocol" => Value::string(images.protocol_name(), span),
                    "shared_memory" => Value::bool(images.uses_shared_memory(), span),
                    "columns" => Value::int(i64::from(image.protocol.size().width), span),
                    "rows" => Value::int(i64::from(image.protocol.size().height), span),
                    "zoom" => Value::float(f64::from(preview.view.zoom), span),
                    "filter" => Value::string(preview.view.filter.name(), span),
                    "pan" => Value::list(
                        vec![
                            Value::int(i64::from(preview.view.pan.0), span),
                            Value::int(i64::from(preview.view.pan.1), span),
                        ],
                        span,
                    ),
                    "id" => match preview.kitty_id {
                        Some(id) => Value::int(i64::from(id), span),
                        None => Value::nothing(span),
                    },
                },
                span,
            ),
            _ => Value::nothing(span),
        };
        Value::record(
            record! {
                "title" => Value::string(&preview.title, span),
                "text" => Value::string(&preview.text, span),
                "scroll" => Value::int(preview.scroll as i64, span),
                "row" => preview.row.clone().unwrap_or_else(|| Value::nothing(span)),
                "context" => session.view_context(id),
                "image" => image,
            },
            span,
        )
    }
}

/// The path a row names, resolved against `cwd`, for the transform's
/// `content_type`.
fn file_path(title: &str, row: &Value, cwd: &Path) -> Option<PathBuf> {
    if title.is_empty() {
        return None;
    }
    let name = row_path(row)?;
    let path = Path::new(&name);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    })
}

/// The pane title for a path: its file name, or the path itself when it
/// has none.
/// The pane's title for a row: its file name, or empty for no row.
fn title_of(row: Option<&Value>) -> String {
    row.and_then(row_path)
        .as_deref()
        .map(file_title)
        .unwrap_or_default()
}

fn file_title(name: &str) -> String {
    Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name)
        .to_string()
}

/// Bytes from a `--data`/`tui image` binary or a closure: an image when
/// they look like one, else text.
fn body_from_bytes(bytes: &[u8]) -> Body {
    if ::image::guess_format(bytes).is_ok() {
        Body::Image(bytes.to_vec())
    } else {
        Body::Text {
            text: String::from_utf8_lossy(bytes).into_owned(),
            transformable: false,
        }
    }
}

/// Read the file named by the row's `name` column (or the row itself when it
/// is a string). Returns the pane title (the file's name) and its body.
fn preview_for_row(row: &Value, max_bytes: usize, cwd: &Path) -> (String, Body) {
    let text = |text: String| Body::Text {
        text,
        transformable: false,
    };
    let Some(name) = row_path(row) else {
        return (String::new(), text("(no path on this row)".into()));
    };
    let path = {
        let p = Path::new(&name);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        }
    };
    let title = file_title(&name);

    let kind = row_type(row);
    if kind.as_deref() == Some("dir") || kind.as_deref() == Some("directory") {
        return (title, text(format!("{name}/\n(directory)")));
    }

    let body = match std::fs::metadata(&path) {
        Ok(meta) if meta.is_dir() => text(format!("{name}/\n(directory)")),
        Ok(meta) if meta.is_file() => read_file_preview(&path, max_bytes, meta.len()),
        Ok(_) => text(format!("{name}\n(not a regular file)")),
        Err(err) => text(if kind.as_deref() == Some("file") {
            format!("cannot read {name}: {err}")
        } else {
            format!("{name}\n({err})")
        }),
    };
    (title, body)
}

/// The pane body for `row`: what the closure returns when it is the
/// source, else the file it names (or the bytes it is), transformed by the
/// closure when there is one. Runs on the UI thread, or on a job's thread
/// with `--threads`.
fn source_body(
    row: Option<&Value>,
    context: &Value,
    wants_row: bool,
    transform: Option<&Closure>,
    engine: Option<(&EngineState, &Stack)>,
    max_bytes: usize,
    cwd: &Path,
) -> Body {
    let body = match (row, wants_row) {
        // One-parameter closure: it is the source, no file is read.
        (Some(_), true) => Body::Text {
            text: String::new(),
            transformable: true,
        },
        (Some(Value::Binary { val, .. }), false) => body_from_bytes(val),
        (Some(row), false) => preview_for_row(row, max_bytes, cwd).1,
        (None, _) => Body::Text {
            text: String::new(),
            transformable: false,
        },
    };
    match (body, transform, engine, row) {
        (
            Body::Text {
                text,
                transformable: true,
            },
            Some(closure),
            Some((engine_state, stack)),
            Some(row),
        ) => {
            let title = row_path(row).as_deref().map(file_title).unwrap_or_default();
            apply_transform(
                engine_state,
                stack,
                closure.clone(),
                wants_row.then_some(row),
                context,
                &text,
                file_path(&title, row, cwd).as_deref(),
            )
        }
        (body, ..) => body,
    }
}

/// The source path and byte count identify a decoded image; a closure's
/// bytes have no path, only their count.
fn image_key(row: Option<&Value>, len: usize) -> String {
    format!("{}:{len}", row.and_then(row_path).unwrap_or_default())
}

/// Give a cloned engine its own signals and job, as `job spawn` does, so a
/// closure on a preview thread can be interrupted and the externals it
/// started killed when the preview moves on. The job is not in the job
/// table: nothing else needs to see it.
fn detach_engine(engine_state: &mut EngineState) -> ThreadJob {
    let signals = Signals::new(Arc::new(AtomicBool::new(false)));
    engine_state.set_signals(signals.clone());
    engine_state.pipeline_externals_state = Arc::new((AtomicU32::new(0), AtomicU32::new(0)));
    engine_state.is_interactive = false;
    engine_state.is_repl = false;
    let (send, recv) = mpsc::channel();
    let job = ThreadJob::new(signals, Some("tui preview".into()), true, send);
    engine_state.current_job = CurrentJob {
        id: JobId::ZERO,
        root: JobId::ZERO,
        background_thread_job: Some(job.clone()),
        mailbox: Arc::new(Mutex::new(Mailbox::new(recv))),
    };
    job
}

/// Run the preview closure. With `row`, the closure is the source and `$in`
/// is empty; otherwise `$in` is `text` with the file's `content_type`. A
/// second parameter gets `context`. Bytes it returns are shown as an image
/// when they look like one.
fn apply_transform(
    engine_state: &EngineState,
    stack: &Stack,
    closure: Closure,
    row: Option<&Value>,
    context: &Value,
    text: &str,
    path: Option<&Path>,
) -> Body {
    let plain = |text: String| Body::Text {
        text,
        transformable: false,
    };
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

    let arity = closure_arity(engine_state, &closure).unwrap_or(0);
    let mut eval = match ClosureEvalOnce::try_new(engine_state, stack, closure, Span::unknown()) {
        Ok(eval) => eval,
        Err(err) => return plain(format!("preview error: {err}")),
    };
    if let Some(row) = row {
        eval = match eval.add_arg(row.clone()) {
            Ok(eval) => eval,
            Err(err) => return plain(format!("preview error: {err}")),
        };
        if arity > 1 {
            eval = match eval.add_arg(context.clone()) {
                Ok(eval) => eval,
                Err(err) => return plain(format!("preview error: {err}")),
            };
        }
    }

    match eval
        .run_with_input(input)
        .and_then(|data| data.into_value(Span::unknown()))
    {
        Ok(Value::String { val, .. }) => plain(val),
        Ok(Value::Binary { val, .. }) => body_from_bytes(&val),
        Ok(Value::Nothing { .. }) => plain(String::new()),
        Ok(other) => plain(other.to_expanded_string("\n", engine_state.get_config())),
        Err(err) => plain(format!("preview error: {err}")),
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
        Value::String { val, .. } | Value::Glob { val, .. } => Some(val.clone()),
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

/// The first `max_bytes` of a text file, with a note when it is binary. An
/// image is read whole (up to [`MAX_IMAGE_BYTES`]) so it can be decoded.
fn read_file_preview(path: &Path, max_bytes: usize, file_len: u64) -> Body {
    let plain = |text: String| Body::Text {
        text,
        transformable: false,
    };
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(err) => return plain(format!("cannot open: {err}")),
    };
    // Read until the cap or the end of the file: one `read` may return
    // less than asked (a pipe, a network file system).
    let mut buf = Vec::with_capacity(max_bytes.min(file_len as usize));
    if let Err(err) = file.take(max_bytes as u64).read_to_end(&mut buf) {
        return plain(format!("cannot read: {err}"));
    }
    let n = buf.len();
    if ::image::guess_format(&buf).is_ok() {
        if file_len > MAX_IMAGE_BYTES {
            return plain(format!("(image, {file_len} bytes: too large to preview)"));
        }
        if file_len as usize > n {
            match std::fs::read(path) {
                Ok(whole) => buf = whole,
                Err(err) => return plain(format!("cannot read: {err}")),
            }
        }
        return Body::Image(buf);
    }
    if buf.contains(&0) {
        return plain(format!("(binary file, {file_len} bytes)"));
    }
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    if file_len as usize > n {
        let _ = write!(text, "\n\n… truncated, showing {n} of {file_len} bytes");
    }
    Body::Text {
        text,
        transformable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::tests::image_fixture;

    fn job(input: Input) -> Job {
        Job {
            key: "img.png:131".into(),
            input,
            shown: None,
            images: Some(Images::headless()),
            options: ImageOptions::default(),
            pane: (40, 20),
            view: View::default(),
            kitty_id: 1,
        }
    }

    #[test]
    fn a_job_decodes_and_encodes_for_the_pane() {
        let token = Arc::new(Cancel::default());
        let loaded = job(Input::Bytes(image_fixture()))
            .run(&Arc::downgrade(&token))
            .expect("result");
        assert_eq!(loaded.text, "image 8x4 png");
        let image = loaded.image.expect("image");
        assert_eq!(image.key, "img.png:131");
        // 8x4 pixels fit in one half-block cell.
        assert_eq!(
            (image.protocol.size().width, image.protocol.size().height),
            (1, 1)
        );
    }

    #[test]
    fn a_job_reports_bytes_that_are_no_image() {
        let token = Arc::new(Cancel::default());
        let loaded = job(Input::Bytes(b"plain text".to_vec()))
            .run(&Arc::downgrade(&token))
            .expect("result");
        assert!(loaded.image.is_none());
        assert_eq!(loaded.text, "(not an image)");
    }

    #[test]
    fn a_job_stops_once_every_handle_is_dropped() {
        let token = Arc::new(Cancel::default());
        let weak = Arc::downgrade(&token);
        drop(token);
        assert!(job(Input::Bytes(image_fixture())).run(&weak).is_none());
    }

    /// A row naming `name` in a fresh directory, as a job's source.
    fn file_source(name: &str, bytes: &[u8]) -> (Source, String) {
        let dir = std::env::temp_dir().join(format!("nu-tui-preview-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("write fixture");
        let path = path.to_string_lossy().into_owned();
        let source = Source {
            row: Some(Value::test_record(
                record! { "name" => Value::test_string(&path) },
            )),
            context: Value::test_nothing(),
            engine: None,
            transform: None,
            wants_row: false,
            max_bytes: 1024,
            cwd: dir,
        };
        (source, path)
    }

    #[test]
    fn a_source_job_reads_text_and_images_on_the_thread() {
        let token = Arc::new(Cancel::default());
        let (source, _) = file_source("notes.txt", b"hello\nworld\n");
        let loaded = job(Input::Source(Box::new(source)))
            .run(&Arc::downgrade(&token))
            .expect("result");
        assert!(loaded.image.is_none());
        assert_eq!(loaded.text, "hello\nworld\n");

        let (source, path) = file_source("pic.png", &image_fixture());
        let loaded = job(Input::Source(Box::new(source)))
            .run(&Arc::downgrade(&token))
            .expect("result");
        assert_eq!(loaded.text, "image 8x4 png");
        assert_eq!(loaded.image.expect("image").key, format!("{path}:131"));

        // Cancelled before it ran: no file is read for nobody.
        let (source, _) = file_source("late.txt", b"x");
        let weak = Arc::downgrade(&token);
        drop(token);
        assert!(job(Input::Source(Box::new(source))).run(&weak).is_none());
    }

    #[test]
    fn dropping_the_last_handle_interrupts_the_engine_job() {
        let mut engine_state = EngineState::new();
        let engine_job = detach_engine(&mut engine_state);
        let signals = engine_state.signals().clone();
        assert!(!signals.interrupted());
        assert!(engine_state.is_background_job());
        let cancel = Arc::new(Cancel {
            engine_job: Some(engine_job),
        });
        let copy = cancel.clone();
        drop(cancel);
        assert!(!signals.interrupted(), "a handle is still held");
        drop(copy);
        assert!(signals.interrupted());
    }

    #[test]
    fn a_pending_image_is_collected_by_polling() {
        let (tx, rx) = mpsc::channel();
        let mut preview = PreviewState {
            text: "(loading the image...)".into(),
            image: Some(ImageState::Loading {
                key: "img.png:131".into(),
                shown: None,
                job: ImageJob {
                    rx: Arc::new(Mutex::new(rx)),
                    _token: Arc::new(Cancel::default()),
                },
            }),
            ..PreviewState::default()
        };
        preview.poll_image(false);
        assert!(matches!(preview.image, Some(ImageState::Loading { .. })));
        assert!(preview.image.as_ref().and_then(ImageState::shown).is_none());

        let token = Arc::new(Cancel::default());
        let loaded = job(Input::Bytes(image_fixture()))
            .run(&Arc::downgrade(&token))
            .expect("result");
        tx.send(loaded).expect("send");
        preview.poll_image(true);
        assert_eq!(preview.text, "image 8x4 png");
        assert!(matches!(preview.image, Some(ImageState::Ready(_))));

        // A job whose thread died without answering clears the pane.
        let (tx, rx) = mpsc::channel::<Loaded>();
        drop(tx);
        preview.image = Some(ImageState::Loading {
            key: "gone".into(),
            shown: None,
            job: ImageJob {
                rx: Arc::new(Mutex::new(rx)),
                _token: Arc::new(Cancel::default()),
            },
        });
        preview.poll_image(false);
        assert!(preview.image.is_none());
        assert_eq!(preview.text, "(the preview could not be loaded)");
    }
}
