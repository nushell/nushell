use super::{
    stream::{StreamManager, StreamManagerHandle},
    test_util::TestCase,
    Interface, InterfaceManager, PluginRead, PluginWrite,
};
use crate::{
    protocol::{
        ExternalStreamInfo, ListStreamInfo, PipelineDataHeader, PluginInput, PluginOutput,
        RawStreamInfo, StreamData, StreamMessage,
    },
    sequence::Sequence,
};
use nu_protocol::{
    DataSource, ListStream, PipelineData, PipelineMetadata, RawStream, ShellError, Span, Value,
};
use std::{path::Path, sync::Arc};

fn test_metadata() -> PipelineMetadata {
    PipelineMetadata {
        data_source: DataSource::FilePath("/test/path".into()),
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
        manager.read_pipeline_data(header, None)?,
        PipelineData::Empty
    ));
    Ok(())
}

#[test]
fn read_pipeline_data_value() -> Result<(), ShellError> {
    let manager = TestInterfaceManager::new(&TestCase::new());
    let value = Value::test_int(4);
    let header = PipelineDataHeader::Value(value.clone());

    match manager.read_pipeline_data(header, None)? {
        PipelineData::Value(read_value, _) => assert_eq!(value, read_value),
        PipelineData::ListStream(_, _) => panic!("unexpected ListStream"),
        PipelineData::ExternalStream { .. } => panic!("unexpected ExternalStream"),
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

    let header = PipelineDataHeader::ListStream(ListStreamInfo { id: 7 });

    let pipe = manager.read_pipeline_data(header, None)?;
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
fn read_pipeline_data_external_stream() -> Result<(), ShellError> {
    let test = TestCase::new();
    let mut manager = TestInterfaceManager::new(&test);

    let iterations = 100;
    let out_pattern = b"hello".to_vec();
    let err_pattern = vec![5, 4, 3, 2];

    test.add(StreamMessage::Data(14, Value::test_int(1).into()));
    for _ in 0..iterations {
        test.add(StreamMessage::Data(
            12,
            StreamData::Raw(Ok(out_pattern.clone())),
        ));
        test.add(StreamMessage::Data(
            13,
            StreamData::Raw(Ok(err_pattern.clone())),
        ));
    }
    test.add(StreamMessage::End(12));
    test.add(StreamMessage::End(13));
    test.add(StreamMessage::End(14));

    let test_span = Span::new(10, 13);
    let header = PipelineDataHeader::ExternalStream(ExternalStreamInfo {
        span: test_span,
        stdout: Some(RawStreamInfo {
            id: 12,
            is_binary: false,
            known_size: Some((out_pattern.len() * iterations) as u64),
        }),
        stderr: Some(RawStreamInfo {
            id: 13,
            is_binary: true,
            known_size: None,
        }),
        exit_code: Some(ListStreamInfo { id: 14 }),
        trim_end_newline: true,
    });

    let pipe = manager.read_pipeline_data(header, None)?;

    // need to consume input
    manager.consume_all()?;

    match pipe {
        PipelineData::ExternalStream {
            stdout,
            stderr,
            exit_code,
            span,
            metadata,
            trim_end_newline,
        } => {
            let stdout = stdout.expect("stdout is None");
            let stderr = stderr.expect("stderr is None");
            let exit_code = exit_code.expect("exit_code is None");
            assert_eq!(test_span, span);
            assert!(
                metadata.is_some(),
                "expected metadata to be Some due to prepare_pipeline_data()"
            );
            assert!(trim_end_newline);

            assert!(!stdout.is_binary);
            assert!(stderr.is_binary);

            assert_eq!(
                Some((out_pattern.len() * iterations) as u64),
                stdout.known_size
            );
            assert_eq!(None, stderr.known_size);

            // check the streams
            let mut count = 0;
            for chunk in stdout.stream {
                assert_eq!(out_pattern, chunk?);
                count += 1;
            }
            assert_eq!(iterations, count, "stdout length");
            let mut count = 0;

            for chunk in stderr.stream {
                assert_eq!(err_pattern, chunk?);
                count += 1;
            }
            assert_eq!(iterations, count, "stderr length");

            assert_eq!(vec![Value::test_int(1)], exit_code.collect::<Vec<_>>());
        }
        _ => panic!("unexpected PipelineData: {pipe:?}"),
    }

    // Don't need to check exactly what was written, just be sure that there is some output
    assert!(test.has_unconsumed_write());

    Ok(())
}

#[test]
fn read_pipeline_data_ctrlc() -> Result<(), ShellError> {
    let manager = TestInterfaceManager::new(&TestCase::new());
    let header = PipelineDataHeader::ListStream(ListStreamInfo { id: 0 });
    let ctrlc = Default::default();
    match manager.read_pipeline_data(header, Some(&ctrlc))? {
        PipelineData::ListStream(
            ListStream {
                ctrlc: stream_ctrlc,
                ..
            },
            _,
        ) => {
            assert!(Arc::ptr_eq(&ctrlc, &stream_ctrlc.expect("ctrlc not set")));
            Ok(())
        }
        _ => panic!("Unexpected PipelineData, should have been ListStream"),
    }
}

#[test]
fn read_pipeline_data_prepared_properly() -> Result<(), ShellError> {
    let manager = TestInterfaceManager::new(&TestCase::new());
    let header = PipelineDataHeader::ListStream(ListStreamInfo { id: 0 });
    match manager.read_pipeline_data(header, None)? {
        PipelineData::ListStream(_, meta) => match meta {
            Some(PipelineMetadata { data_source }) => match data_source {
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

    let (header, writer) = interface.init_write_pipeline_data(PipelineData::Empty, &())?;

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
        interface.init_write_pipeline_data(PipelineData::Value(value.clone(), None), &())?;

    match header {
        PipelineDataHeader::Value(read_value) => assert_eq!(value, read_value),
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

    match interface.init_write_pipeline_data(PipelineData::Value(value, None), &()) {
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
    let pipe = PipelineData::ListStream(
        ListStream::from_stream(values.clone().into_iter(), None),
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
fn write_pipeline_data_external_stream() -> Result<(), ShellError> {
    let test = TestCase::new();
    let manager = TestInterfaceManager::new(&test);
    let interface = manager.get_interface();

    let stdout_bufs = vec![
        b"hello".to_vec(),
        b"world".to_vec(),
        b"these are tests".to_vec(),
    ];
    let stdout_len = stdout_bufs.iter().map(|b| b.len() as u64).sum::<u64>();
    let stderr_bufs = vec![b"error messages".to_vec(), b"go here".to_vec()];
    let exit_code = Value::test_int(7);

    let span = Span::new(400, 500);

    // Set up pipeline data for an external stream
    let pipe = PipelineData::ExternalStream {
        stdout: Some(RawStream::new(
            Box::new(stdout_bufs.clone().into_iter().map(Ok)),
            None,
            span,
            Some(stdout_len),
        )),
        stderr: Some(RawStream::new(
            Box::new(stderr_bufs.clone().into_iter().map(Ok)),
            None,
            span,
            None,
        )),
        exit_code: Some(ListStream::from_stream(
            std::iter::once(exit_code.clone()),
            None,
        )),
        span,
        metadata: None,
        trim_end_newline: true,
    };

    let (header, writer) = interface.init_write_pipeline_data(pipe, &())?;

    let info = match header {
        PipelineDataHeader::ExternalStream(info) => info,
        _ => panic!("unexpected header: {header:?}"),
    };

    writer.write()?;

    let stdout_info = info.stdout.as_ref().expect("stdout info is None");
    let stderr_info = info.stderr.as_ref().expect("stderr info is None");
    let exit_code_info = info.exit_code.as_ref().expect("exit code info is None");

    assert_eq!(span, info.span);
    assert!(info.trim_end_newline);

    assert_eq!(Some(stdout_len), stdout_info.known_size);
    assert_eq!(None, stderr_info.known_size);

    // Now make sure the stream messages have been written
    let mut stdout_iter = stdout_bufs.into_iter();
    let mut stderr_iter = stderr_bufs.into_iter();
    let mut exit_code_iter = std::iter::once(exit_code);

    let mut stdout_ended = false;
    let mut stderr_ended = false;
    let mut exit_code_ended = false;

    // There's no specific order these messages must come in with respect to how the streams are
    // interleaved, but all of the data for each stream must be in its original order, and the
    // End must come after all Data
    for msg in test.written() {
        match msg {
            PluginOutput::Data(id, data) => {
                if id == stdout_info.id {
                    let result: Result<Vec<u8>, ShellError> =
                        data.try_into().expect("wrong data in stdout stream");
                    assert_eq!(
                        stdout_iter.next().expect("too much data in stdout"),
                        result.expect("unexpected error in stdout stream")
                    );
                } else if id == stderr_info.id {
                    let result: Result<Vec<u8>, ShellError> =
                        data.try_into().expect("wrong data in stderr stream");
                    assert_eq!(
                        stderr_iter.next().expect("too much data in stderr"),
                        result.expect("unexpected error in stderr stream")
                    );
                } else if id == exit_code_info.id {
                    let code: Value = data.try_into().expect("wrong data in stderr stream");
                    assert_eq!(
                        exit_code_iter.next().expect("too much data in stderr"),
                        code
                    );
                } else {
                    panic!("unrecognized stream id: {id}");
                }
            }
            PluginOutput::End(id) => {
                if id == stdout_info.id {
                    assert!(!stdout_ended, "double End of stdout");
                    assert!(stdout_iter.next().is_none(), "unexpected end of stdout");
                    stdout_ended = true;
                } else if id == stderr_info.id {
                    assert!(!stderr_ended, "double End of stderr");
                    assert!(stderr_iter.next().is_none(), "unexpected end of stderr");
                    stderr_ended = true;
                } else if id == exit_code_info.id {
                    assert!(!exit_code_ended, "double End of exit_code");
                    assert!(
                        exit_code_iter.next().is_none(),
                        "unexpected end of exit_code"
                    );
                    exit_code_ended = true;
                } else {
                    panic!("unrecognized stream id: {id}");
                }
            }
            other => panic!("unexpected output: {other:?}"),
        }
    }

    assert!(stdout_ended, "stdout did not End");
    assert!(stderr_ended, "stderr did not End");
    assert!(exit_code_ended, "exit_code did not End");

    Ok(())
}
