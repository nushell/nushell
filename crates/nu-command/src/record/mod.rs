//! Subcommands of `record` that take a record and return a new record.
//!
//! Each subcommand also takes a list of records, such as a table, and streams
//! one output record per input record.
//!
//! | Command | Description |
//! |---|---|
//! | [`RecordCommand`] | Display this help message |
//! | [`RecordWhere`] | Keep the fields for which a closure returns true |
//! | [`RecordEach`] | Rebuild the record from the entries a closure returns |
//! | [`RecordWalk`] | Run a closure on every nested value, passing its cell path |
//! | [`RecordApply`] | Run a record of closures on the matching fields |

mod apply;
mod each;
mod record_;
mod walk;
mod where_;

pub use apply::RecordApply;
pub use each::RecordEach;
pub use record_::RecordCommand;
pub use walk::RecordWalk;
pub use where_::RecordWhere;

use nu_protocol::{IntoPipelineData, PipelineData, Record, ShellError, Signals, Span, Value};

/// Run `f` on each record in the input of a `record` subcommand.
///
/// `f` receives each record with its span and returns the output record.
///
/// A single record gives a single record, and an error from `f` is the
/// command's error. A list or stream of records is processed lazily, one record
/// at a time, into a list stream: an error from `f`, or an item that is not a
/// record, becomes an error value in that item's place and the stream goes on,
/// as with `rename`. The pipeline metadata is passed through.
fn map_records(
    input: PipelineData,
    head: Span,
    signals: &Signals,
    mut f: impl FnMut(Record, Span) -> Result<Record, ShellError> + Send + 'static,
) -> Result<PipelineData, ShellError> {
    // `PipelineData::map` reads a byte stream to the end, and an external
    // command's output can be endless. It can't hold records, so reject it unread.
    if matches!(input, PipelineData::ByteStream(..)) {
        return Err(input.unsupported_input_error("record", head));
    }

    input.map(
        move |value| {
            let span = value.span();
            match value {
                Value::Record { val, .. } => match f(val.into_owned(), span) {
                    Ok(record) => Value::record(record, span),
                    Err(error) => Value::error(error, span),
                },
                Value::Error { .. } => value,
                other => Value::error(
                    other
                        .into_pipeline_data()
                        .unsupported_input_error("record", head),
                    span,
                ),
            }
        },
        signals,
    )
}
