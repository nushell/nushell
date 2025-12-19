use super::super::{
    config::ExploreConfig,
    nu_common::{NuText, run_command_with_value},
    pager::{
        Frame, Transition, ViewInfo,
        report::{Report, Severity},
    },
    views::{Layout, Orientation, Preview, RecordView, View, ViewConfig},
};
use super::ViewCommand;
use anyhow::Result;
use crossterm::event::KeyEvent;
use nu_engine::get_columns;
use nu_protocol::{
    PipelineData, Value,
    engine::{EngineState, Stack},
};
use ratatui::layout::Rect;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

#[derive(Debug, Default, Clone)]
pub struct NuCmd {
    command: String,
}

impl NuCmd {
    pub fn new() -> Self {
        Self {
            command: String::new(),
        }
    }

    pub const NAME: &'static str = "nu";
}

impl ViewCommand for NuCmd {
    type View = NuView;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn description(&self) -> &'static str {
        ""
    }

    fn parse(&mut self, args: &str) -> Result<()> {
        args.trim().clone_into(&mut self.command);

        Ok(())
    }

    fn spawn(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        value: Option<Value>,
        config: &ViewConfig,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();

        // Clone what we need for the background thread
        let engine_state = engine_state.clone();
        let mut stack = stack.clone();
        let command = self.command.clone();
        let explore_config = config.explore_config.clone();

        // Create channel for communicating results
        let (sender, receiver) = mpsc::channel();

        // Spawn background thread to run the command
        let handle = thread::spawn(move || {
            stream_command(
                &command,
                &value,
                &engine_state,
                &mut stack,
                &explore_config,
                sender,
            );
        });

        Ok(NuView {
            state: ViewState::Loading,
            receiver: Some(receiver),
            _handle: Some(handle),
            command_text: self.command.clone(),
            explore_config: config.explore_config.clone(),
            frame_count: 0,
            // Streaming state
            columns: Vec::new(),
            rows: Vec::new(),
            is_record: false,
            stream_done: false,
            last_row_count: 0,
        })
    }
}

/// Messages sent from the background thread to the UI
enum StreamMessage {
    /// Column names (sent once, when first determined)
    Columns(Vec<String>),
    /// A batch of rows
    Rows(Vec<Vec<Value>>),
    /// This is a record (single row with orientation left)
    IsRecord,
    /// Simple value result (not a table)
    SimpleValue(String),
    /// Streaming is complete
    Done,
    /// An error occurred
    Error(String),
}

/// Run the nu command and stream results back via the channel
fn stream_command(
    command: &str,
    value: &Value,
    engine_state: &EngineState,
    stack: &mut Stack,
    _explore_config: &ExploreConfig,
    sender: mpsc::Sender<StreamMessage>,
) {
    let pipeline = match run_command_with_value(command, value, engine_state, stack) {
        Ok(p) => p,
        Err(e) => {
            let _ = sender.send(StreamMessage::Error(format!("Command failed: {e}")));
            return;
        }
    };

    match pipeline {
        PipelineData::Empty => {
            let _ = sender.send(StreamMessage::Done);
        }
        PipelineData::Value(Value::Record { val, .. }, ..) => {
            // Record - show with left orientation
            let _ = sender.send(StreamMessage::IsRecord);
            let (cols, vals): (Vec<_>, Vec<_>) = val.into_owned().into_iter().unzip();
            let _ = sender.send(StreamMessage::Columns(cols));
            if !vals.is_empty() {
                let _ = sender.send(StreamMessage::Rows(vec![vals]));
            }
            let _ = sender.send(StreamMessage::Done);
        }
        PipelineData::Value(Value::List { vals, .. }, ..) => {
            // List value - stream it
            stream_values(vals.into_iter(), &sender);
        }
        PipelineData::Value(Value::String { val, .. }, ..) => {
            // String - show as preview
            let _ = sender.send(StreamMessage::SimpleValue(val));
        }
        PipelineData::Value(value, ..) => {
            // Other simple value - show as preview
            let text = value.to_abbreviated_string(&engine_state.config);
            let _ = sender.send(StreamMessage::SimpleValue(text));
        }
        PipelineData::ListStream(stream, ..) => {
            // Stream values as they arrive
            stream_values(stream.into_iter(), &sender);
        }
        PipelineData::ByteStream(stream, ..) => {
            // ByteStream - collect to string and show as preview
            let span = stream.span();
            match stream.into_string() {
                Ok(text) => {
                    let _ = sender.send(StreamMessage::SimpleValue(text));
                }
                Err(e) => {
                    let _ = sender.send(StreamMessage::Error(format!(
                        "Failed to read stream: {}",
                        Value::error(e, span).to_debug_string()
                    )));
                }
            }
        }
    }
}

/// Stream values from an iterator, sending rows in batches
fn stream_values<I>(iter: I, sender: &mpsc::Sender<StreamMessage>)
where
    I: Iterator<Item = Value>,
{
    const BATCH_SIZE: usize = 1;
    const INITIAL_ROWS_FOR_COLUMNS: usize = 1;

    let mut columns: Option<Vec<String>> = None;
    let mut batch: Vec<Vec<Value>> = Vec::with_capacity(BATCH_SIZE);
    let mut initial_values: Vec<Value> = Vec::new();

    for value in iter {
        if columns.is_none() {
            // Buffer initial values to determine columns
            initial_values.push(value);

            if initial_values.len() >= INITIAL_ROWS_FOR_COLUMNS {
                // Determine columns from buffered values
                let cols = get_columns(&initial_values);
                if !cols.is_empty() {
                    let _ = sender.send(StreamMessage::Columns(cols.clone()));
                }

                // Convert buffered values to rows and send
                let rows: Vec<Vec<Value>> = initial_values
                    .drain(..)
                    .map(|v| value_to_row(&cols, &v))
                    .collect();

                if sender.send(StreamMessage::Rows(rows)).is_err() {
                    return; // Receiver dropped
                }

                columns = Some(cols);
            }
        } else {
            // We have columns, add to batch
            let cols = columns.as_ref().unwrap();
            batch.push(value_to_row(cols, &value));

            if batch.len() >= BATCH_SIZE {
                if sender
                    .send(StreamMessage::Rows(std::mem::take(&mut batch)))
                    .is_err()
                {
                    return; // Receiver dropped
                }
                batch = Vec::with_capacity(BATCH_SIZE);
            }
        }
    }

    // Handle case where we never got enough values to determine columns
    if columns.is_none() && !initial_values.is_empty() {
        let cols = get_columns(&initial_values);
        if !cols.is_empty() {
            let _ = sender.send(StreamMessage::Columns(cols.clone()));
        }

        let rows: Vec<Vec<Value>> = initial_values
            .drain(..)
            .map(|v| value_to_row(&cols, &v))
            .collect();

        let _ = sender.send(StreamMessage::Rows(rows));
    }

    // Send any remaining batch
    if !batch.is_empty() {
        let _ = sender.send(StreamMessage::Rows(batch));
    }

    let _ = sender.send(StreamMessage::Done);
}

/// Convert a Value to a row based on expected columns
fn value_to_row(cols: &[String], value: &Value) -> Vec<Value> {
    if cols.is_empty() {
        vec![value.clone()]
    } else if let Value::Record { val, .. } = value {
        cols.iter()
            .map(|col| val.get(col).cloned().unwrap_or_default())
            .collect()
    } else {
        // Non-record value in a list - put it in the first column
        let mut row = vec![Value::default(); cols.len()];
        if !row.is_empty() {
            row[0] = value.clone();
        }
        row
    }
}

/// The current state of the view
enum ViewState {
    /// Waiting for first data
    Loading,
    /// Streaming/showing a RecordView
    Records(Box<RecordView>),
    /// Showing a simple preview
    Preview(Preview),
    /// Command completed with no output
    Empty,
    /// An error occurred
    Error(String),
}

/// A view that runs a command in the background and streams results
pub struct NuView {
    state: ViewState,
    receiver: Option<Receiver<StreamMessage>>,
    _handle: Option<JoinHandle<()>>,
    command_text: String,
    explore_config: ExploreConfig,
    frame_count: usize,
    // Streaming state - used to accumulate data before/while view exists
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
    is_record: bool,
    stream_done: bool,
    last_row_count: usize,
}

impl NuView {
    /// Process any pending messages from the background thread
    fn process_messages(&mut self) {
        // Take receiver temporarily to avoid borrow issues
        let receiver = match self.receiver.take() {
            Some(r) => r,
            None => return,
        };

        // Process all available messages
        let mut should_update_view = false;
        loop {
            match receiver.try_recv() {
                Ok(StreamMessage::Columns(cols)) => {
                    self.columns = cols;
                }
                Ok(StreamMessage::Rows(new_rows)) => {
                    self.rows.extend(new_rows);
                    should_update_view = true;
                }
                Ok(StreamMessage::IsRecord) => {
                    self.is_record = true;
                }
                Ok(StreamMessage::SimpleValue(text)) => {
                    self.state = ViewState::Preview(Preview::new(&text));
                    self.stream_done = true;
                    // Don't put receiver back - we're done
                    return;
                }
                Ok(StreamMessage::Done) => {
                    self.stream_done = true;
                    // If we have no data yet, mark as empty
                    if self.rows.is_empty()
                        && !matches!(self.state, ViewState::Records(_) | ViewState::Preview(_))
                    {
                        self.state = ViewState::Empty;
                    }
                    // Don't put receiver back - we're done
                    if should_update_view {
                        self.update_record_view();
                    }
                    return;
                }
                Ok(StreamMessage::Error(e)) => {
                    self.state = ViewState::Error(e);
                    self.stream_done = true;
                    // Don't put receiver back - we're done
                    return;
                }
                Err(TryRecvError::Empty) => {
                    // No more messages available right now
                    // Put receiver back for next time
                    self.receiver = Some(receiver);
                    if should_update_view {
                        self.update_record_view();
                    }
                    return;
                }
                Err(TryRecvError::Disconnected) => {
                    // Thread finished
                    self.stream_done = true;
                    if self.rows.is_empty()
                        && !matches!(self.state, ViewState::Records(_) | ViewState::Preview(_))
                    {
                        self.state = ViewState::Empty;
                    }
                    // Don't put receiver back - we're done
                    if should_update_view {
                        self.update_record_view();
                    }
                    return;
                }
            }
        }
    }

    /// Create or update the RecordView with current data
    fn update_record_view(&mut self) {
        if self.rows.is_empty() {
            return;
        }

        let cols = if self.columns.is_empty() {
            vec![String::new()]
        } else {
            self.columns.clone()
        };

        match &mut self.state {
            ViewState::Records(existing_view) => {
                // Append new rows to existing view
                let layer = existing_view.get_top_layer_mut();
                layer.record_values = self.rows.clone();
                // Update cursor limits
                layer
                    .cursor
                    .y
                    .view
                    .set_size(layer.record_values.len())
                    .unwrap();
                layer
                    .cursor
                    .x
                    .view
                    .set_size(layer.column_names.len())
                    .unwrap();
                // Invalidate text to force redraw
                layer.record_text = None;
            }
            _ => {
                // Create new view with all accumulated data
                let mut view =
                    RecordView::new(cols, self.rows.clone(), self.explore_config.clone());

                if self.is_record {
                    view.set_top_layer_orientation(Orientation::Left);
                }

                self.state = ViewState::Records(Box::new(view));
            }
        }
    }

    fn spinner_char(&self) -> char {
        const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        SPINNER[self.frame_count % SPINNER.len()]
    }

    fn row_count(&self) -> usize {
        self.rows.len()
    }

    fn is_streaming(&self) -> bool {
        !self.stream_done
    }
}

impl View for NuView {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        self.frame_count = self.frame_count.wrapping_add(1);

        // Tail the view if new rows have been added during streaming
        if self.rows.len() > self.last_row_count {
            self.last_row_count = self.rows.len();
            if let ViewState::Records(view) = &mut self.state {
                view.tail(area.width as u16, area.height as u16);
            }
        }

        match &mut self.state {
            ViewState::Loading => {
                // Don't draw anything in the content area while loading
                // The status bar will show the spinner
            }
            ViewState::Records(view) => {
                view.draw(f, area, cfg, layout);
            }
            ViewState::Preview(view) => {
                view.draw(f, area, cfg, layout);
            }
            ViewState::Empty => {
                // Nothing to display
            }
            ViewState::Error(_) => {
                // Error is shown in status bar
            }
        }
    }

    fn handle_input(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        layout: &Layout,
        info: &mut ViewInfo,
        key: KeyEvent,
    ) -> Transition {
        match &mut self.state {
            ViewState::Records(view) => view.handle_input(engine_state, stack, layout, info, key),
            ViewState::Preview(view) => view.handle_input(engine_state, stack, layout, info, key),
            _ => Transition::None,
        }
    }

    fn update(&mut self, info: &mut ViewInfo) -> bool {
        // Process any pending messages from the stream
        self.process_messages();

        // Update the status bar based on current state
        match &self.state {
            ViewState::Loading => {
                let spinner = self.spinner_char();
                let msg = format!("{} Running: {}", spinner, self.command_text);
                info.status = Some(Report::message(msg, Severity::Info));
                true // Keep polling
            }
            ViewState::Records(_) => {
                let row_count = self.row_count();
                if self.is_streaming() {
                    let spinner = self.spinner_char();
                    let msg = format!("{} Streaming: {} rows", spinner, row_count);
                    info.status = Some(Report::message(msg, Severity::Info));
                    true // Keep polling
                } else {
                    info.status = Some(Report::new(
                        format!("{} rows", row_count),
                        Severity::Info,
                        String::new(),
                        String::new(),
                        String::new(),
                    ));
                    false // Done polling
                }
            }
            ViewState::Preview(_) => {
                info.status = Some(Report::message("Preview", Severity::Info));
                false // Done polling
            }
            ViewState::Empty => {
                info.status = Some(Report::message("No output", Severity::Info));
                false // Done polling
            }
            ViewState::Error(msg) => {
                info.status = Some(Report::error(msg.clone()));
                false // Done polling
            }
        }
    }

    fn show_data(&mut self, i: usize) -> bool {
        match &mut self.state {
            ViewState::Records(view) => view.show_data(i),
            ViewState::Preview(view) => view.show_data(i),
            _ => false,
        }
    }

    fn collect_data(&self) -> Vec<NuText> {
        match &self.state {
            ViewState::Records(view) => view.collect_data(),
            ViewState::Preview(view) => view.collect_data(),
            _ => Vec::new(),
        }
    }

    fn exit(&mut self) -> Option<Value> {
        match &mut self.state {
            ViewState::Records(view) => view.exit(),
            ViewState::Preview(view) => view.exit(),
            _ => None,
        }
    }
}
