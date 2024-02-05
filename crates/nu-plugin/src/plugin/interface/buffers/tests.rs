use std::sync::Arc;

use nu_protocol::{ShellError, Span, Value};

use crate::protocol::{ExternalStreamInfo, ListStreamInfo, PipelineDataHeader, PluginData, RawStreamInfo, StreamData};

use super::{StreamBuffer, StreamBuffers, TypedStreamBuffer};

#[test]
fn stream_buffers_get_non_existing_is_err() {
    let mut buffers = StreamBuffers::default();
    buffers.get(0).expect_err("should be an error");
}

#[test]
fn stream_buffers_insert_existing_is_err() {
    let mut buffers = StreamBuffers {
        streams: vec![(0, Box::new(TypedStreamBuffer::new_list()))],
    };
    buffers
        .insert(0, TypedStreamBuffer::new_list())
        .expect_err("should be an error");
}

#[test]
fn stream_buffers_cleanup() {
    let mut buffers = StreamBuffers {
        streams: vec![
            (0, Box::new(TypedStreamBuffer::new_list())),
            (
                1,
                Box::new(TypedStreamBuffer::Raw(StreamBuffer {
                    queue: None,
                    ended: true,
                })),
            ),
            (
                2,
                Box::new(TypedStreamBuffer::Raw(StreamBuffer {
                    queue: Some(Default::default()),
                    ended: true,
                })),
            ),
            (
                3,
                Box::new(TypedStreamBuffer::Raw(StreamBuffer {
                    queue: Some(vec![Ok(vec![0])].into()),
                    ended: true,
                })),
            ),
            (
                4,
                Box::new(TypedStreamBuffer::Raw(StreamBuffer {
                    queue: Some(vec![Ok(vec![0])].into()),
                    ended: false,
                })),
            ),
        ],
    };
    buffers.cleanup();
    assert!(buffers.get(0).is_ok(), "cleaned up the wrong stream");
    assert!(buffers.get(1).is_err(), "failed to clean up");
    assert!(buffers.get(2).is_err(), "failed to clean up");
    assert!(buffers.get(3).is_ok(), "cleaned up the wrong stream");
    assert!(buffers.get(4).is_ok(), "cleaned up the wrong stream");
}

#[test]
fn stream_buffers_init_stream_non_stream_header() -> Result<(), ShellError> {
    let mut buffers = StreamBuffers::default();
    buffers.init_stream(&PipelineDataHeader::Empty)?;
    assert!(
        buffers.streams.is_empty(),
        "stream was created erroneously for Empty"
    );
    buffers.init_stream(&PipelineDataHeader::Value(Value::test_bool(true)))?;
    assert!(
        buffers.streams.is_empty(),
        "stream was created erroneously for Value"
    );
    buffers.init_stream(&PipelineDataHeader::PluginData(PluginData {
        name: None,
        data: vec![],
        span: Span::test_data(),
    }))?;
    assert!(
        buffers.streams.is_empty(),
        "stream was created erroneously for PluginData"
    );
    Ok(())
}

#[test]
fn stream_buffers_init_stream_list() -> Result<(), ShellError> {
    let mut buffers = StreamBuffers::default();

    buffers.init_stream(&PipelineDataHeader::ListStream(ListStreamInfo { id: 4 }))?;

    match buffers.get(4)? {
        TypedStreamBuffer::List(_) => (),
        TypedStreamBuffer::Raw(_) => panic!("init_stream created wrong type"),
    }
    Ok(())
}

#[test]
fn stream_buffers_init_stream_external() -> Result<(), ShellError> {
    let mut buffers = StreamBuffers::default();

    buffers.init_stream(&PipelineDataHeader::ExternalStream(ExternalStreamInfo {
        span: Span::test_data(),
        stdout: Some(RawStreamInfo {
            id: 1,
            is_binary: false,
            known_size: None,
        }),
        stderr: Some(RawStreamInfo {
            id: 2,
            is_binary: false,
            known_size: None,
        }),
        exit_code: Some(ListStreamInfo { id: 3 }),
        trim_end_newline: false,
    }))?;

    match buffers.get(1)? {
        TypedStreamBuffer::List(_) => panic!("init_stream created wrong type"),
        TypedStreamBuffer::Raw(buf) => assert!(!buf.is_dropped()),
    }
    match buffers.get(2)? {
        TypedStreamBuffer::List(_) => panic!("init_stream created wrong type"),
        TypedStreamBuffer::Raw(buf) => assert!(!buf.is_dropped()),
    }
    match buffers.get(3)? {
        TypedStreamBuffer::List(buf) => assert!(!buf.is_dropped()),
        TypedStreamBuffer::Raw(_) => panic!("init_stream created wrong type"),
    }
    Ok(())
}

#[test]
fn stream_buffers_skip() -> Result<(), ShellError> {
    let mut buffers = StreamBuffers {
        streams: vec![
            (1, Box::new(TypedStreamBuffer::new_list())),
            (2, Box::new(TypedStreamBuffer::new_list())),
        ],
    };
    buffers.skip(2, StreamData::List(Some(Value::test_int(4))))?;
    assert!(matches!(buffers.get(2)?.pop_list()?, Some(Some(_))));
    buffers.skip(1, StreamData::List(Some(Value::test_int(5))))?;
    assert!(matches!(buffers.get(1)?.pop_list()?, Some(Some(_))));

    buffers
        .skip(4, StreamData::Raw(Some(Ok(vec![]))))
        .expect_err("trying to write to a non-existent stream should have been an error");
    Ok(())
}

#[test]
fn typed_stream_buffer_list_accepts_only_list_stream_data() {
    let mut buffers = TypedStreamBuffer::new_list();

    buffers
        .push_back(StreamData::List(Some(Value::test_bool(true))))
        .expect("list was not accepted");

    buffers
        .push_back(StreamData::Raw(Some(Ok(vec![]))))
        .expect_err("raw was accepted");
}

#[test]
fn stream_buffers_raw_accepts_only_raw_stream_data() {
    let mut buffers = TypedStreamBuffer::new_raw();

    buffers
        .push_back(StreamData::List(Some(Value::test_bool(true))))
        .expect_err("list was accepted");

    buffers
        .push_back(StreamData::Raw(Some(Ok(vec![]))))
        .expect("raw was not accepted");
}

#[test]
fn typed_stream_buffer_list_is_fully_consumed() -> Result<(), ShellError> {
    let mut buffers = TypedStreamBuffer::new_list();
    assert!(!buffers.is_fully_consumed());
    buffers.push_back(StreamData::List(None))?;
    assert!(buffers.is_fully_consumed());
    Ok(())
}

#[test]
fn typed_stream_buffer_external_raw_is_fully_consumed() -> Result<(), ShellError> {
    let mut buffers = TypedStreamBuffer::new_raw();
    assert!(!buffers.is_fully_consumed(), "initial state");
    buffers.push_back(StreamData::Raw(None))?;
    assert!(buffers.is_fully_consumed());
    Ok(())
}

#[test]
fn stream_buffer_push_pop() -> Result<(), ShellError> {
    let mut buffer = StreamBuffer::new();
    buffer.push_back(Some(1))?;
    buffer.push_back(Some(2))?;
    assert_eq!(buffer.pop_front()?, Some(Some(1)));
    assert_eq!(buffer.pop_front()?, Some(Some(2)));
    assert_eq!(buffer.pop_front()?, None);
    buffer.push_back(Some(42))?;
    assert_eq!(buffer.pop_front()?, Some(Some(42)));
    assert_eq!(buffer.pop_front()?, None);
    buffer.push_back(None)?;
    assert_eq!(buffer.pop_front()?, Some(None));
    assert_eq!(buffer.pop_front()?, Some(None));
    Ok(())
}

#[test]
fn stream_buffer_write_after_end_err() -> Result<(), ShellError> {
    let mut buffer = StreamBuffer::new();
    buffer.push_back(Some(1))?;
    buffer.push_back(None)?;
    buffer
        .push_back(Some(2))
        .expect_err("write after end succeeded");
    Ok(())
}

#[test]
fn stream_buffer_is_fully_consumed() -> Result<(), ShellError> {
    let mut buffer = StreamBuffer::new();
    assert!(
        !buffer.is_fully_consumed(),
        "default state is fully consumed: {buffer:?}"
    );
    buffer.push_back(Some(1))?;
    assert!(
        !buffer.is_fully_consumed(),
        "fully consumed after pushing Some: {buffer:?}"
    );
    buffer.pop_front()?;
    assert!(
        !buffer.is_fully_consumed(),
        "fully consumed after popping: {buffer:?}"
    );
    buffer.push_back(Some(1))?;
    buffer.push_back(None)?;
    assert!(
        !buffer.is_fully_consumed(),
        "fully consumed after pushing None: {buffer:?}"
    );
    buffer.pop_front()?;
    assert!(
        buffer.is_fully_consumed(),
        "not fully consumed after last message: {buffer:?}"
    );
    Ok(())
}

#[test]
fn stream_buffer_dropped_is_fully_consumed() {
    assert!(!StreamBuffer::<()> {
        queue: None,
        ended: false
    }
    .is_fully_consumed());
    assert!(StreamBuffer::<()> {
        queue: None,
        ended: true
    }
    .is_fully_consumed());
}

#[test]
fn stream_buffer_dropped_push() {
    // Use an Arc and a Weak copy of it as an indicator of whether the data is still alive
    let data = Arc::new(1);
    let data_weak = Arc::downgrade(&data);
    let mut dropped = StreamBuffer::new();
    dropped.set_dropped();
    dropped
        .push_back(Some(data))
        .expect("can't push on dropped");
    // Should still be dropped - i.e., the message is not stored
    assert!(dropped.is_dropped());
    // Pushing none should set the ended flag
    dropped.push_back(None).expect("can't push on dropped");
    assert!(dropped.is_dropped());
    // The data itself should also have been dropped - i.e., there are no copies of it around
    assert!(data_weak.upgrade().is_none(), "dropped data was preserved");
}

#[test]
fn stream_buffer_dropped_pop_err() {
    let mut buffer = StreamBuffer::<()>::new();
    buffer.set_dropped();
    buffer.pop_front().expect_err("should be an error");
}
