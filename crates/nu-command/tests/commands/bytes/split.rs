use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Signals, Span};
use nu_test_support::prelude::*;
use std::sync::{Arc, atomic::AtomicBool};

#[test]
fn split_stream_matches_across_reads_and_discards_unfinished_field_on_error() -> Result {
    let span = Span::test_data();
    let error = ShellError::AccessEmptyContent { span };
    let stream = ByteStream::from_result_iter(
        [
            Ok(b"lefta".as_slice()),
            Ok(b"aaunclosed".as_slice()),
            Err(error.clone()),
        ],
        span,
        Signals::empty(),
        ByteStreamType::Binary,
    );
    let mut rows = test()
        .run_raw_with_data("bytes split 'aa'", PipelineData::byte_stream(stream, None))?
        .body
        .into_iter();
    assert_eq!(rows.next().unwrap().as_binary()?, b"left");
    assert_eq!(rows.next().unwrap().unwrap_error().unwrap_err(), error);
    assert!(rows.next().is_none());
    Ok(())
}

#[test]
fn split_stream_reports_cancellation_after_a_completed_field() -> Result {
    let span = Span::test_data();
    let signals = Signals::new(Arc::new(AtomicBool::new(false)));
    let chunks = std::iter::once_with({
        let signals = signals.clone();
        move || {
            signals.trigger();
            b"left--unclosed".as_slice()
        }
    });
    let stream = ByteStream::from_iter(chunks, span, signals, ByteStreamType::Binary);
    let mut rows = test()
        .run_raw_with_data("bytes split '--'", PipelineData::byte_stream(stream, None))?
        .body
        .into_iter();
    assert_eq!(rows.next().unwrap().as_binary()?, b"left");
    assert_eq!(
        rows.next().unwrap().unwrap_error().unwrap_err(),
        ShellError::Interrupted { span },
    );
    assert!(rows.next().is_none());
    Ok(())
}
