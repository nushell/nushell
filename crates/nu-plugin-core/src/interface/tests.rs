use super::{
    Interface, InterfaceManager, PluginRead, PluginWrite,
    stream::{StreamManager, StreamManagerHandle},
    test_util::TestCase,
};
use nu_plugin_protocol::{
    ByteStreamInfo, ListStreamInfo, PipelineDataHeader, PluginInput, PluginOutput, StreamData,
    StreamMessage,
};
use nu_protocol::{
    ByteStream, ByteStreamSource, ByteStreamType, DataSource, ListStream, PipelineData,
    PipelineMetadata, ShellError, Signals, Span, Value, engine::Sequence, shell_error::io::IoError,
};
use std::{path::Path, sync::Arc};

fn test_metadata() -> PipelineMetadata {
    PipelineMetadata {
        data_source: DataSource::FilePath("/test/path".into()),
        ..Default::default()
    }
}

#[derive(Debug)]
struct TestInterfaceManager {
    stream_manager: StreamManager,
    test: TestCase<PluginInput, PluginOutput>,
    seq: Arc<Sequence>,
}

#[derive(Debug, Clone)]
struct TestInterface {
    stream_manager_handle: StreamManagerHandle,
    test: TestCase<PluginInput, PluginOutput>,
    seq: Arc<Sequence>,
}

impl TestInterfaceManager {
    fn new(test: &TestCase<PluginInput, PluginOutput>) -> TestInterfaceManager {
        TestInterfaceManager {
            stream_manager: StreamManager::new(),
            test: test.clone(),
            seq: Arc::new(Sequence::default()),
        }
    }

    fn consume_all(&mut self) -> Result<(), ShellError> {
        while let Some(msg) = self.test.read()? {
            self.consume(msg)?;
        }
        Ok(())
    }
}

impl InterfaceManager for TestInterfaceManager {
    type Interface = TestInterface;
    type Input = PluginInput;

    fn get_interface(&self) -> Self::Interface {
        TestInterface {
            stream_manager_handle: self.stream_manager.get_handle(),
            test: self.test.clone(),
            seq: self.seq.clone(),
        }
    }

    fn consume(&mut self, input: Self::Input) -> Result<(), ShellError> {
        match input {
            PluginInput::Data(..)
            | PluginInput::End(..)
            | PluginInput::Drop(..)
            | PluginInput::Ack(..) => self.consume_stream_message(
                input
                    .try_into()
                    .expect("failed to convert message to StreamMessage"),
            ),
            _ => unimplemented!(),
        }
    }

    fn stream_manager(&self) -> &StreamManager {
        &self.stream_manager
    }

    fn prepare_pipeline_data(&self, data: PipelineData) -> Result<PipelineData, ShellError> {
        Ok(data.set_metadata(Some(test_metadata())))
    }
}

impl Interface for TestInterface {
    type Output = PluginOutput;
    type DataContext = ();

    fn write(&self, output: Self::Output) -> Result<(), ShellError> {
        self.test.write(&output)
    }

    fn flush(&self) -> Result<(), ShellError> {
        Ok(())
    }

    fn stream_id_sequence(&self) -> &Sequence {
        &self.seq
    }

    fn stream_manager_handle(&self) -> &StreamManagerHandle {
        &self.stream_manager_handle
    }

    fn prepare_pipeline_data(
        &self,
        data: PipelineData,
        _context: &(),
    ) -> Result<PipelineData, ShellError> {
        // Add an arbitrary check to the data to verify this is being called
        match data {
            PipelineData::Value(Value::Binary { .. }, None) => Err(ShellError::NushellFailed {
                msg: "TEST can't send binary".into(),
            }),
            _ => Ok(data),
        }
    }
}

#[test]
fn read_pipeline_data_empty() -> Result<(), ShellError> {
    let manager = TestInterfaceManager::new(&TestCase::new());
    let header = PipelineDataHeader::Empty;

    assert!(matches!(
        manager.read_pipeline_data(header, &Signals::empty())?,
        PipelineData::Empty
    ));
    Ok(())
}

#[test]
fn read_pipeline_data_value() -> Result<(), ShellError> {
    let manager = TestInterfaceManager::new(&TestCase::new());
    let value = Value::test_int(4);
    let metadata = Some(PipelineMetadata {
        data_source: DataSource::FilePath("/test/path".into()),
        ..Default::default()
    });
    let header = PipelineDataHeader::Value(value.clone(), metadata.clone());
    match manager.read_pipeline_data(header, &Signals::empty())? {
        PipelineData::Value(read_value, read_metadata) => {
            assert_eq!(value, read_value);
            assert_eq!(metadata, read_metadata);
        }
        PipelineData::ListStream(..) => panic!("unexpected ListStream"),
        PipelineData::ByteStream(..) => panic!("unexpected ByteStream"),
        PipelineData::Empty => panic!("unexpected Empty"),
    }

    Ok(())
}

#[test]
fn read_pipeline_data_list_stream() -> Result<(), ShellError> {
    let test = TestCase::new();
    let mut manager = TestInterfaceManager::new(&test);

    let data = (0..100).map(Value::test_int).collect::<Vec<_>>();

    for value in &data {
        test.add(StreamMessage::Data(7, value.clone().into()));
    }
    test.add(StreamMessage::End(7));

    let metadata = Some(PipelineMetadata {
        content_type: Some("foobar".into()),
        ..Default::default()
    });

    let header = PipelineDataHeader::ListStream(ListStreamInfo {
        id: 7,
        span: Span::test_data(),
        metadata,
    });

    let pipe = manager.read_pipeline_data(header, &Signals::empty())?;
    assert!(
        matches!(pipe, PipelineData::ListStream(..)),
        "unexpected PipelineData: {pipe:?}"
    );

    // need to consume input
    manager.consume_all()?;

    let mut count = 0;
    for (expected, read) in data.into_iter().zip(pipe) {
        assert_eq!(expected, read);
        count += 1;
    }
    assert_eq!(100, count);

    assert!(test.has_unconsumed_write());

    Ok(())
}

#[test]
fn read_pipeline_data_byte_stream() -> Result<(), ShellError> {
    let test = TestCase::new();
    let mut manager = TestInterfaceManager::new(&test);

    let iterations = 100;
    let out_pattern = b"hello".to_vec();

    for _ in 0..iterations {
        test.add(StreamMessage::Data(
            12,
            StreamData::Raw(Ok(out_pattern.clone())),
        ));
    }
    test.add(StreamMessage::End(12));

    let test_span = Span::new(10, 13);

    let metadata = Some(PipelineMetadata {
        content_type: Some("foobar".into()),
        ..Default::default()
    });

    let header = PipelineDataHeader::ByteStream(ByteStreamInfo {
        id: 12,
        span: test_span,
        type_: ByteStreamType::Unknown,
        metadata,
    });

    let pipe = manager.read_pipeline_data(header, &Signals::empty())?;

    // need to consume input
    manager.consume_all()?;

    match pipe {
        PipelineData::ByteStream(stream, metadata) => {
            assert_eq!(test_span, stream.span());
            assert!(
                metadata.is_some(),
                "expected metadata to be Some due to prepare_pipeline_data()"
            );

            match stream.into_source() {
                ByteStreamSource::Read(mut read) => {
                    let mut buf = Vec::new();
                    read.read_to_end(&mut buf)
                        .map_err(|err| IoError::new(err, test_span, None))?;
                    let iter = buf.chunks_exact(out_pattern.len());
                    assert_eq!(iter.len(), iterations);
                    for chunk in iter {
                        assert_eq!(out_pattern, chunk)
                    }
                }
                ByteStreamSource::File(..) => panic!("unexpected byte stream source: file"),
                ByteStreamSource::Child(..) => {
                    panic!("unexpected byte stream source: child")
                }
            }
        }
        _ => panic!("unexpected PipelineData: {pipe:?}"),
    }

    // Don't need to check exactly what was written, just be sure that there is some output
    assert!(test.has_unconsumed_write());

    Ok(())
}

#[test]
fn read_pipeline_data_prepared_properly() -> Result<(), ShellError> {
    let manager = TestInterfaceManager::new(&TestCase::new());
    let metadata = Some(PipelineMetadata {
        content_type: Some("foobar".into()),
        ..Default::default()
    });

    let header = PipelineDataHeader::ListStream(ListStreamInfo {
        id: 0,
        span: Span::test_data(),
        metadata,
    });
    match manager.read_pipeline_data(header, &Signals::empty())? {
        PipelineData::ListStream(_, meta) => match meta {
            Some(PipelineMetadata { data_source, .. }) => match data_source {
                DataSource::FilePath(path) => {
                    assert_eq!(Path::new("/test/path"), path);
                    Ok(())
                }
                _ => panic!("wrong metadata: {data_source:?}"),
            },
            None => panic!("metadata not set"),
        },
        _ => panic!("Unexpected PipelineData, should have been ListStream"),
    }
}

#[test]
fn write_pipeline_data_empty() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = TestInterfaceManager::new(&test);
    let interface = manager.get_interface();

    let (header, writer) = interface.init_write_pipeline_data(PipelineData::empty(), &())?;

    assert!(matches!(header, PipelineDataHeader::Empty));

    writer.write()?;

    assert!(
        !test.has_unconsumed_write(),
        "Empty shouldn't write any stream messages, test: {test:#?}"
    );

    Ok(())
}

#[test]
fn write_pipeline_data_value() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = TestInterfaceManager::new(&test);
    let interface = manager.get_interface();
    let value = Value::test_int(7);

    let (header, writer) =
        interface.init_write_pipeline_data(PipelineData::value(value.clone(), None), &())?;

    match header {
        PipelineDataHeader::Value(read_value, _) => assert_eq!(value, read_value),
        _ => panic!("unexpected header: {header:?}"),
    }

    writer.write()?;

    assert!(
        !test.has_unconsumed_write(),
        "Value shouldn't write any stream messages, test: {test:#?}"
    );

    Ok(())
}

#[test]
fn write_pipeline_data_prepared_properly() {
    let manager = TestInterfaceManager::new(&TestCase::new());
    let interface = manager.get_interface();

    // Sending a binary should be an error in our test scenario
    let value = Value::test_binary(vec![7, 8]);

    match interface.init_write_pipeline_data(PipelineData::value(value, None), &()) {
        Ok(_) => panic!("prepare_pipeline_data was not called"),
        Err(err) => {
            assert_eq!(
                ShellError::NushellFailed {
                    msg: "TEST can't send binary".into()
                }
                .to_string(),
                err.to_string()
            );
        }
    }
}

#[test]
fn write_pipeline_data_list_stream() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = TestInterfaceManager::new(&test);
    let interface = manager.get_interface();

    let values = vec![
        Value::test_int(40),
        Value::test_bool(false),
        Value::test_string("this is a test"),
    ];

    // Set up pipeline data for a list stream
    let pipe = PipelineData::list_stream(
        ListStream::new(
            values.clone().into_iter(),
            Span::test_data(),
            Signals::empty(),
        ),
        None,
    );

    let (header, writer) = interface.init_write_pipeline_data(pipe, &())?;

    let info = match header {
        PipelineDataHeader::ListStream(info) => info,
        _ => panic!("unexpected header: {header:?}"),
    };

    writer.write()?;

    // Now make sure the stream messages have been written
    for value in values {
        match test.next_written().expect("unexpected end of stream") {
            PluginOutput::Data(id, data) => {
                assert_eq!(info.id, id, "Data id");
                match data {
                    StreamData::List(read_value) => assert_eq!(value, read_value, "Data value"),
                    _ => panic!("unexpected Data: {data:?}"),
                }
            }
            other => panic!("unexpected output: {other:?}"),
        }
    }

    match test.next_written().expect("unexpected end of stream") {
        PluginOutput::End(id) => {
            assert_eq!(info.id, id, "End id");
        }
        other => panic!("unexpected output: {other:?}"),
    }

    assert!(!test.has_unconsumed_write());

    Ok(())
}

#[test]
fn write_pipeline_data_byte_stream() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = TestInterfaceManager::new(&test);
    let interface = manager.get_interface();

    let expected = "hello\nworld\nthese are tests";
    let span = Span::new(400, 500);

    // Set up pipeline data for a byte stream
    let data = PipelineData::byte_stream(
        ByteStream::read(
            std::io::Cursor::new(expected),
            span,
            Signals::empty(),
            ByteStreamType::Unknown,
        ),
        None,
    );

    let (header, writer) = interface.init_write_pipeline_data(data, &())?;

    let info = match header {
        PipelineDataHeader::ByteStream(info) => info,
        _ => panic!("unexpected header: {header:?}"),
    };

    writer.write()?;

    assert_eq!(span, info.span);

    // Now make sure the stream messages have been written
    let mut actual = Vec::new();
    let mut ended = false;

    for msg in test.written() {
        match msg {
            PluginOutput::Data(id, data) => {
                if id == info.id {
                    let data: Result<Vec<u8>, ShellError> =
                        data.try_into().expect("wrong data in stream");

                    let data = data.expect("unexpected error in stream");
                    actual.extend(data);
                } else {
                    panic!("unrecognized stream id: {id}");
                }
            }
            PluginOutput::End(id) => {
                if id == info.id {
                    ended = true;
                } else {
                    panic!("unrecognized stream id: {id}");
                }
            }
            other => panic!("unexpected output: {other:?}"),
        }
    }

    assert_eq!(expected.as_bytes(), actual);
    assert!(ended, "stream did not End");

    Ok(())
}
