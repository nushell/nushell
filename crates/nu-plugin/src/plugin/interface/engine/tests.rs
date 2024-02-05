use nu_protocol::{CustomValue, ListStream, PipelineData, RawStream, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

use crate::plugin::interface::engine::EngineInterfaceIo;
use crate::plugin::interface::stream_data_io::{
    gen_stream_data_tests, StreamBuffer, StreamBuffers, StreamDataIo,
};
use crate::plugin::interface::test_util::TestCase;
use crate::protocol::{
    CallInfo, CallInput, EvaluatedCall, ExternalStreamInfo, PluginCall, PluginData, PluginInput,
    PluginOutput, RawStreamInfo,
};
use crate::{PluginCallResponse, StreamData};

use super::EngineInterface;

gen_stream_data_tests!(
    PluginInput(add_input),
    PluginOutput(next_written_output),
    |test| test.engine_interface_impl()
);

#[test]
fn read_call_signature() {
    let test = TestCase::new();
    test.add_input(PluginInput::Call(PluginCall::Signature));

    match test.engine_interface().read_call().unwrap() {
        Some(PluginCall::Signature) => (),
        Some(other) => panic!("read unexpected call: {:?}", other),
        None => panic!("end of input"),
    }
}

#[test]
fn read_call_run() {
    let test = TestCase::new();
    let call_info = CallInfo {
        name: "test call".into(),
        call: EvaluatedCall {
            head: Span::test_data(),
            positional: vec![],
            named: vec![],
        },
        input: CallInput::Empty,
        config: None,
    };
    test.add_input(PluginInput::Call(PluginCall::Run(call_info.clone())));

    match test.engine_interface().read_call().unwrap() {
        Some(PluginCall::Run(read_call_info)) => {
            assert_eq!(call_info.name, read_call_info.name);
            assert_eq!(call_info.call.head, read_call_info.call.head);
            assert_eq!(call_info.call.positional, read_call_info.call.positional);
            assert_eq!(call_info.call.named, read_call_info.call.named);
            assert_eq!(call_info.input, read_call_info.input);
            assert_eq!(call_info.config, read_call_info.config);
        }
        Some(other) => panic!("read unexpected call: {:?}", other),
        None => panic!("end of input"),
    }
}

#[test]
fn read_call_collapse_custom_value() {
    let test = TestCase::new();
    let data = PluginData {
        data: vec![42, 13, 37],
        span: Span::test_data(),
    };
    test.add_input(PluginInput::Call(PluginCall::CollapseCustomValue(
        data.clone(),
    )));

    match test.engine_interface().read_call().unwrap() {
        Some(PluginCall::CollapseCustomValue(read_data)) => assert_eq!(data, read_data),
        Some(other) => panic!("read unexpected call: {:?}", other),
        None => panic!("end of input"),
    }
}

#[test]
fn read_call_unexpected_stream_data() {
    let test = TestCase::new();
    test.add_input(PluginInput::StreamData(StreamData::List(None)));
    test.add_input(PluginInput::Call(PluginCall::Signature));

    test.engine_interface()
        .read_call()
        .expect_err("should be an error");
}

#[test]
fn read_call_ignore_dropped_stream_data() {
    let test = TestCase::new();
    test.add_input(PluginInput::StreamData(StreamData::List(None)));
    test.add_input(PluginInput::Call(PluginCall::Signature));

    let interface = test.engine_interface_impl();
    interface.read.lock().unwrap().1.list = StreamBuffer::Dropped;
    interface.read_call().expect("should succeed");
}

fn test_call_with_input(input: CallInput) -> CallInfo {
    CallInfo {
        name: "test call".into(),
        call: EvaluatedCall {
            head: Span::test_data(),
            positional: vec![],
            named: vec![],
        },
        input,
        config: None,
    }
}

fn dbg<T>(val: T) -> String
where
    T: std::fmt::Debug,
{
    format!("{:?}", val)
}

fn validate_stream_data_acceptance(input: CallInput, accepts: [bool; 4]) {
    let test = TestCase::new();
    let call_info = test_call_with_input(input);
    test.add_input(PluginInput::Call(PluginCall::Run(call_info)));

    let interface = test.engine_interface_impl();

    interface.read_call().expect("call failed");

    let data_types = [
        StreamData::List(Some(Value::test_bool(true))),
        StreamData::ExternalStdout(Some(Ok(vec![]))),
        StreamData::ExternalStderr(Some(Ok(vec![]))),
        StreamData::ExternalExitCode(Some(Value::test_int(1))),
    ];

    for (data, accept) in data_types.iter().zip(accepts) {
        test.clear_input();
        test.add_input(PluginInput::StreamData(data.clone()));
        let result = match data {
            StreamData::List(_) => interface.read_list().map(dbg),
            StreamData::ExternalStdout(_) => interface.read_external_stdout().map(dbg),
            StreamData::ExternalStderr(_) => interface.read_external_stderr().map(dbg),
            StreamData::ExternalExitCode(_) => interface.read_external_exit_code().map(dbg),
        };
        match result {
            Ok(success) if !accept => {
                panic!("{data:?} was successfully consumed, but shouldn't have been: {success}")
            }
            Err(err) if accept => {
                panic!("{data:?} was rejected, but should have been accepted: {err}")
            }
            _ => (),
        }
    }
}

#[test]
fn read_call_run_with_empty_input_doesnt_accept_stream_data() {
    validate_stream_data_acceptance(CallInput::Empty, [false; 4])
}

#[test]
fn read_call_run_with_value_input_doesnt_accept_stream_data() {
    validate_stream_data_acceptance(CallInput::Value(Value::test_int(4)), [false; 4])
}

#[test]
fn read_call_run_with_list_stream_input_accepts_only_list_stream_data() {
    validate_stream_data_acceptance(
        CallInput::ListStream,
        [
            true, // list stream
            false, false, false,
        ],
    )
}

#[test]
fn read_call_run_with_external_stream_stdout_input_accepts_only_external_stream_stdout_data() {
    let call_input = CallInput::ExternalStream(ExternalStreamInfo {
        span: Span::test_data(),
        stdout: Some(RawStreamInfo {
            is_binary: false,
            known_size: None,
        }),
        stderr: None,
        has_exit_code: false,
        trim_end_newline: false,
    });

    validate_stream_data_acceptance(
        call_input,
        [
            false, true, // external stdout
            false, false,
        ],
    )
}

#[test]
fn read_call_run_with_external_stream_stderr_input_accepts_only_external_stream_stderr_data() {
    let call_input = CallInput::ExternalStream(ExternalStreamInfo {
        span: Span::test_data(),
        stdout: None,
        stderr: Some(RawStreamInfo {
            is_binary: false,
            known_size: None,
        }),
        has_exit_code: false,
        trim_end_newline: false,
    });

    validate_stream_data_acceptance(
        call_input,
        [
            false, false, true, // external stderr
            false,
        ],
    )
}

#[test]
fn read_call_run_with_external_stream_exit_code_input_accepts_only_external_stream_exit_code_data()
{
    let call_input = CallInput::ExternalStream(ExternalStreamInfo {
        span: Span::test_data(),
        stdout: None,
        stderr: None,
        has_exit_code: true,
        trim_end_newline: false,
    });

    validate_stream_data_acceptance(
        call_input,
        [
            false, false, false, true, // external exit code
        ],
    )
}

#[test]
fn read_call_run_with_external_stream_all_input_accepts_only_all_external_stream_data() {
    let call_input = CallInput::ExternalStream(ExternalStreamInfo {
        span: Span::test_data(),
        stdout: Some(RawStreamInfo {
            is_binary: false,
            known_size: None,
        }),
        stderr: Some(RawStreamInfo {
            is_binary: false,
            known_size: None,
        }),
        has_exit_code: true,
        trim_end_newline: false,
    });

    validate_stream_data_acceptance(
        call_input,
        [
            false, true, // external stdout
            true, // external stderr
            true, // external exit code
        ],
    )
}

#[test]
fn read_call_end_of_input() {
    let test = TestCase::new();
    if let Some(call) = test.engine_interface().read_call().expect("should succeed") {
        panic!("should have been end of input, but read {call:?}");
    }
}

#[test]
fn read_call_io_error() {
    let test = TestCase::new();
    test.set_read_error(ShellError::IOError {
        msg: "test error".into(),
    });

    match test
        .engine_interface()
        .read_call()
        .expect_err("should be an error")
    {
        ShellError::IOError { msg } if msg == "test error" => (),
        other => panic!("got some other error: {other}"),
    }
}

#[test]
fn write_call_response() {
    let test = TestCase::new();
    let response = PluginCallResponse::Empty;
    test.engine_interface()
        .write_call_response(response.clone())
        .expect("should succeed");
    match test.next_written_output() {
        Some(PluginOutput::CallResponse(PluginCallResponse::Empty)) => (),
        Some(other) => panic!("wrote the wrong message: {other:?}"),
        None => panic!("didn't write anything"),
    }
    assert!(!test.has_unconsumed_write());
}

#[test]
fn write_call_response_error() {
    let test = TestCase::new();
    test.set_write_error(ShellError::IOError {
        msg: "test error".into(),
    });

    let response = PluginCallResponse::Empty;
    match test
        .engine_interface()
        .write_call_response(response)
        .expect_err("should be an error")
    {
        ShellError::IOError { msg } if msg == "test error" => (),
        other => panic!("got some other error: {other}"),
    }
    assert!(!test.has_unconsumed_write());
}

#[test]
fn make_pipeline_data_empty() {
    let test = TestCase::new();

    let pipe = test
        .engine_interface()
        .make_pipeline_data(CallInput::Empty)
        .expect("can't make pipeline data");

    match pipe {
        PipelineData::Empty => (),
        PipelineData::Value(_, _) => panic!("got value, expected empty"),
        PipelineData::ListStream(_, _) => panic!("got list stream"),
        PipelineData::ExternalStream { .. } => panic!("got external stream"),
    }
}

#[test]
fn make_pipeline_data_value() {
    let test = TestCase::new();

    let value = Value::test_int(2);
    let pipe = test
        .engine_interface()
        .make_pipeline_data(CallInput::Value(value.clone()))
        .expect("can't make pipeline data");

    match pipe {
        PipelineData::Empty => panic!("got empty, expected value"),
        PipelineData::Value(v, _) => assert_eq!(value, v),
        PipelineData::ListStream(_, _) => panic!("got list stream"),
        PipelineData::ExternalStream { .. } => panic!("got external stream"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MyCustom(i32);

#[typetag::serde]
impl CustomValue for MyCustom {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn value_string(&self) -> String {
        self.0.to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::int(self.0 as i64, span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn make_pipeline_data_custom_data() {
    let test = TestCase::new();

    let custom: Box<dyn CustomValue> = Box::new(MyCustom(42));
    let bincoded = bincode::serialize(&custom).expect("serialization failed");

    let data = PluginData {
        data: bincoded,
        span: Span::test_data(),
    };
    let call_input = CallInput::Data(data);

    let pipe = test
        .engine_interface()
        .make_pipeline_data(call_input)
        .expect("failed to make pipeline data");

    match pipe {
        PipelineData::Empty => panic!("got empty, expected value"),
        PipelineData::Value(v, _) => {
            let read_custom = v.as_custom_value().expect("not a custom value");
            let read_downcast: &MyCustom = read_custom.as_any().downcast_ref().expect("wrong type");
            assert_eq!(&MyCustom(42), read_downcast);
        }
        PipelineData::ListStream(_, _) => panic!("got list stream"),
        PipelineData::ExternalStream { .. } => panic!("got external stream"),
    }
}

#[test]
fn make_pipeline_data_list_stream() {
    let test = TestCase::new();

    let values = [Value::test_int(4), Value::test_string("hello")];

    for value in &values {
        test.add_input(PluginInput::StreamData(StreamData::List(Some(
            value.clone(),
        ))));
    }
    // end
    test.add_input(PluginInput::StreamData(StreamData::List(None)));

    let call_input = CallInput::ListStream;

    let interface = EngineInterface::from({
        let interface = test.engine_interface_impl();
        interface.read.lock().unwrap().1 = StreamBuffers::new_list();
        interface
    });

    let pipe = interface
        .make_pipeline_data(call_input)
        .expect("failed to make pipeline data");

    assert!(matches!(pipe, PipelineData::ListStream(..)));

    for (defined_value, read_value) in values.into_iter().zip(pipe.into_iter()) {
        assert_eq!(defined_value, read_value);
    }
}

#[test]
fn make_pipeline_data_external_stream() {
    let test = TestCase::new();

    // Test many simultaneous streams out of order
    let stream_data = [
        StreamData::ExternalStdout(Some(Ok(b"foo".to_vec()))),
        StreamData::ExternalStderr(Some(Ok(b"bar".to_vec()))),
        StreamData::ExternalExitCode(Some(Value::test_int(1))),
        StreamData::ExternalStderr(Some(Ok(b"barr".to_vec()))),
        StreamData::ExternalStderr(None),
        StreamData::ExternalStdout(Some(Ok(b"fooo".to_vec()))),
        StreamData::ExternalStdout(None),
        StreamData::ExternalExitCode(None),
    ];

    for data in stream_data {
        test.add_input(PluginInput::StreamData(data));
    }

    let call_input = CallInput::ExternalStream(ExternalStreamInfo {
        span: Span::test_data(),
        stdout: Some(RawStreamInfo {
            is_binary: true,
            known_size: Some(7),
        }),
        stderr: Some(RawStreamInfo {
            is_binary: false,
            known_size: None,
        }),
        has_exit_code: true,
        trim_end_newline: false,
    });

    let interface = EngineInterface::from({
        let interface = test.engine_interface_impl();
        interface.read.lock().unwrap().1 = StreamBuffers::new_external(true, true, true);
        interface
    });

    let pipe = interface
        .make_pipeline_data(call_input)
        .expect("failed to make pipeline data");

    match pipe {
        PipelineData::ExternalStream {
            stdout,
            stderr,
            exit_code,
            span,
            trim_end_newline,
            ..
        } => {
            assert!(stdout.is_some());
            assert!(stderr.is_some());
            assert!(exit_code.is_some());
            assert_eq!(Span::test_data(), span, "span");
            assert!(!trim_end_newline);

            if let Some(rs) = stdout.as_ref() {
                assert!(rs.is_binary, "stdout.is_binary=false");
                assert_eq!(Some(7), rs.known_size, "stdout.known_size");
            }
            if let Some(rs) = stderr.as_ref() {
                assert!(!rs.is_binary, "stderr.is_binary=false");
                assert_eq!(None, rs.known_size, "stderr.known_size");
            }

            let out_bytes = stdout.unwrap().into_bytes().expect("failed to read stdout");
            let err_bytes = stderr.unwrap().into_bytes().expect("failed to read stderr");
            let exit_code_vals: Vec<_> = exit_code.unwrap().collect();

            assert_eq!(b"foofooo", &out_bytes.item[..]);
            assert_eq!(b"barbarr", &err_bytes.item[..]);
            assert_eq!(vec![Value::test_int(1)], exit_code_vals);
        }
        PipelineData::Empty => panic!("expected external stream, got empty"),
        PipelineData::Value(..) => panic!("expected external stream, got value"),
        PipelineData::ListStream(..) => panic!("expected external stream, got list stream"),
    }
}

#[test]
fn make_pipeline_data_external_stream_error() {
    let test = TestCase::new();

    // Just test stdout, but with an error
    let spec_msg = "failure";
    let stream_data = [
        StreamData::ExternalExitCode(Some(Value::int(1, Span::test_data()))),
        StreamData::ExternalStdout(Some(Err(ShellError::NushellFailed {
            msg: spec_msg.into(),
        }))),
        StreamData::ExternalStdout(None),
    ];

    for data in stream_data {
        test.add_input(PluginInput::StreamData(data));
    }

    // Still enable the other streams, to ensure ignoring the other data works
    let call_input = CallInput::ExternalStream(ExternalStreamInfo {
        span: Span::test_data(),
        stdout: Some(RawStreamInfo {
            is_binary: false,
            known_size: None,
        }),
        stderr: Some(RawStreamInfo {
            is_binary: false,
            known_size: None,
        }),
        has_exit_code: true,
        trim_end_newline: false,
    });

    let interface = EngineInterface::from({
        let interface = test.engine_interface_impl();
        interface.read.lock().unwrap().1 = StreamBuffers::new_external(true, true, true);
        interface
    });

    let pipe = interface
        .make_pipeline_data(call_input)
        .expect("failed to make pipeline data");

    match pipe {
        PipelineData::ExternalStream {
            stdout,
            stderr,
            exit_code,
            ..
        } => {
            assert!(stdout.is_some());
            assert!(stderr.is_some());
            assert!(exit_code.is_some());

            match stdout
                .unwrap()
                .into_bytes()
                .expect_err("stdout read successfully")
            {
                ShellError::NushellFailed { msg } => assert_eq!(spec_msg, msg),
                other => panic!("unexpected other error while reading stdout: {other}"),
            }
        }
        PipelineData::Empty => panic!("expected external stream, got empty"),
        PipelineData::Value(..) => panic!("expected external stream, got value"),
        PipelineData::ListStream(..) => panic!("expected external stream, got list stream"),
    }
}

#[test]
fn write_pipeline_data_response_empty() {
    let test = TestCase::new();
    test.engine_interface()
        .write_pipeline_data_response(PipelineData::Empty)
        .expect("failed to write empty response");

    match test.next_written_output() {
        Some(output) => match output {
            PluginOutput::CallResponse(PluginCallResponse::Empty) => (),
            PluginOutput::CallResponse(other) => panic!("unexpected response: {other:?}"),
            other => panic!("unexpected output: {other:?}"),
        },
        None => panic!("no response written"),
    }

    assert!(!test.has_unconsumed_write());
}

#[test]
fn write_pipeline_data_response_value() {
    let test = TestCase::new();
    let value = Value::test_string("hello");
    let data = PipelineData::Value(value.clone(), None);
    test.engine_interface()
        .write_pipeline_data_response(data)
        .expect("failed to write value response");

    match test.next_written_output() {
        Some(output) => match output {
            PluginOutput::CallResponse(PluginCallResponse::Value(v)) => assert_eq!(value, *v),
            PluginOutput::CallResponse(other) => panic!("unexpected response: {other:?}"),
            other => panic!("unexpected output: {other:?}"),
        },
        None => panic!("no response written"),
    }

    assert!(!test.has_unconsumed_write());
}

#[test]
fn write_pipeline_data_response_list_stream() {
    let test = TestCase::new();

    let values = vec![
        Value::test_int(4),
        Value::test_bool(false),
        Value::test_string("foobar"),
    ];

    let list_stream = ListStream::from_stream(values.clone().into_iter(), None);
    let data = PipelineData::ListStream(list_stream, None);
    test.engine_interface()
        .write_pipeline_data_response(data)
        .expect("failed to write list stream response");

    // Response starts by signaling a ListStream return value:
    match test.next_written_output() {
        Some(PluginOutput::CallResponse(PluginCallResponse::ListStream)) => (),
        Some(other) => panic!("unexpected response: {other:?}"),
        None => panic!("response not written"),
    }

    // Followed by each stream value...
    for (expected_value, output) in values.into_iter().zip(test.written_outputs()) {
        match output {
            PluginOutput::StreamData(StreamData::List(Some(read_value))) => {
                assert_eq!(expected_value, read_value)
            }
            PluginOutput::StreamData(StreamData::List(None)) => {
                panic!("unexpected early end of stream")
            }
            other => panic!("unexpected other output: {other:?}"),
        }
    }

    // Followed by List(None) to end the stream
    match test.next_written_output() {
        Some(PluginOutput::StreamData(StreamData::List(None))) => (),
        Some(other) => panic!("expected list end, unexpected output: {other:?}"),
        None => panic!("missing list stream end signal"),
    }

    assert!(!test.has_unconsumed_write());
}

#[test]
fn write_pipeline_data_response_external_stream_stdout_only() {
    let test = TestCase::new();

    let stdout_chunks = vec![b"nushel".to_vec(), b"l rock".to_vec(), b"s!\n".to_vec()];

    let stdout_raw_stream = RawStream::new(
        Box::new(stdout_chunks.clone().into_iter().map(Ok)),
        None,
        Span::test_data(),
        None,
    );

    let span = Span::new(1000, 1050);

    let data = PipelineData::ExternalStream {
        stdout: Some(stdout_raw_stream),
        stderr: None,
        exit_code: None,
        span,
        metadata: None,
        trim_end_newline: false,
    };

    test.engine_interface()
        .write_pipeline_data_response(data)
        .expect("failed to write external stream pipeline data");

    // First, there should be a header telling us metadata about the external stream
    match test.next_written_output() {
        Some(PluginOutput::CallResponse(PluginCallResponse::ExternalStream(info))) => {
            assert_eq!(span, info.span, "info.span");
            match info.stdout {
                Some(RawStreamInfo {
                    is_binary,
                    known_size,
                }) => {
                    let _ = is_binary; // undefined, could be anything
                    assert_eq!(None, known_size);
                }
                None => todo!(),
            }
            assert!(info.stderr.is_none(), "info.stderr: {:?}", info.stderr);
            assert!(!info.has_exit_code, "info.has_exit_code=true");
            assert!(!info.trim_end_newline, "info.trim_end_newline=true");
        }
        Some(other) => panic!("unexpected response written: {other:?}"),
        None => panic!("no response written"),
    }

    // Then, just check the outputs. They should be in exactly the same order with nothing extra
    for expected_chunk in stdout_chunks {
        match test.next_written_output() {
            Some(PluginOutput::StreamData(StreamData::ExternalStdout(option))) => {
                let read_chunk = option
                    .transpose()
                    .expect("error in stdout stream")
                    .expect("early EOF signal in stdout stream");
                assert_eq!(expected_chunk, read_chunk);
            }
            Some(other) => panic!("unexpected output: {other:?}"),
            None => panic!("unexpected end of output"),
        }
    }

    // And there should be an end of stream signal (`Ok(None)`)
    match test.next_written_output() {
        Some(PluginOutput::StreamData(StreamData::ExternalStdout(option))) => match option {
            Some(Ok(data)) => panic!("unexpected extra data on stdout stream: {data:?}"),
            Some(Err(err)) => panic!("unexpected error at end of stdout stream: {err}"),
            None => (),
        },
        Some(other) => panic!("unexpected output: {other:?}"),
        None => panic!("unexpected end of output"),
    }

    assert!(!test.has_unconsumed_write());
}

#[test]
fn write_pipeline_data_response_external_stream_stdout_err() {
    let test = TestCase::new();

    let spec_msg = "something bad";
    let spec_val_span = Span::new(1090, 1100);
    let spec_call_span = Span::new(1000, 1030);

    let error = ShellError::IncorrectValue {
        msg: spec_msg.into(),
        val_span: spec_val_span,
        call_span: spec_call_span,
    };

    let stdout_raw_stream = RawStream::new(
        Box::new(std::iter::once(Err(error))),
        None,
        Span::test_data(),
        None,
    );

    let data = PipelineData::ExternalStream {
        stdout: Some(stdout_raw_stream),
        stderr: None,
        exit_code: None,
        span: Span::test_data(),
        metadata: None,
        trim_end_newline: false,
    };

    test.engine_interface()
        .write_pipeline_data_response(data)
        .expect("failed to write external stream pipeline data");

    // Check response header
    match test.next_written_output() {
        Some(PluginOutput::CallResponse(PluginCallResponse::ExternalStream(info))) => {
            assert!(info.stdout.is_some(), "info.stdout is not present");
            assert!(info.stderr.is_none(), "info.stderr: {:?}", info.stderr);
            assert!(!info.has_exit_code, "info.has_exit_code=true");
        }
        Some(other) => panic!("unexpected response written: {other:?}"),
        None => panic!("no response written"),
    }

    // Check error
    match test.next_written_output() {
        Some(PluginOutput::StreamData(StreamData::ExternalStdout(Some(result)))) => match result {
            Ok(value) => panic!("unexpected value in stream: {value:?}"),
            Err(ShellError::IncorrectValue {
                msg,
                val_span,
                call_span,
            }) => {
                assert_eq!(spec_msg, msg, "msg");
                assert_eq!(spec_val_span, val_span, "val_span");
                assert_eq!(spec_call_span, call_span, "call_span");
            }
            Err(err) => panic!("unexpected other error on stream: {err}"),
        },
        Some(other) => panic!("unexpected output: {other:?}"),
        None => panic!("didn't write the exit code"),
    }

    // Check end of stream
    match test.next_written_output() {
        Some(PluginOutput::StreamData(StreamData::ExternalStdout(None))) => (),
        Some(other) => panic!("unexpected output: {other:?}"),
        None => panic!("didn't write the exit code end of stream signal"),
    }

    assert!(!test.has_unconsumed_write());
}

#[test]
fn write_pipeline_data_response_external_stream_exit_code_only() {
    let test = TestCase::new();

    let exit_code_stream = ListStream::from_stream(std::iter::once(Value::test_int(0)), None);

    let data = PipelineData::ExternalStream {
        stdout: None,
        stderr: None,
        exit_code: Some(exit_code_stream),
        span: Span::test_data(),
        metadata: None,
        trim_end_newline: false,
    };

    test.engine_interface()
        .write_pipeline_data_response(data)
        .expect("failed to write external stream pipeline data");

    // Check response header
    match test.next_written_output() {
        Some(PluginOutput::CallResponse(PluginCallResponse::ExternalStream(info))) => {
            // just check what matters here, the other tests cover other bits
            assert!(info.stdout.is_none(), "info.stdout: {:?}", info.stdout);
            assert!(info.stderr.is_none(), "info.stderr: {:?}", info.stderr);
            assert!(info.has_exit_code);
        }
        Some(other) => panic!("unexpected response: {other:?}"),
        None => panic!("didn't write any response"),
    }

    // Check exit code value
    match test.next_written_output() {
        Some(PluginOutput::StreamData(StreamData::ExternalExitCode(Some(value)))) => {
            assert_eq!(Value::test_int(0), value);
        }
        Some(other) => panic!("unexpected output: {other:?}"),
        None => panic!("didn't write the exit code"),
    }

    // Check end of stream
    match test.next_written_output() {
        Some(PluginOutput::StreamData(StreamData::ExternalExitCode(None))) => (),
        Some(other) => panic!("unexpected output: {other:?}"),
        None => panic!("didn't write the exit code end of stream signal"),
    }

    assert!(!test.has_unconsumed_write());
}

#[test]
fn write_pipeline_data_response_external_stream_full() {
    let test = TestCase::new();

    // Consume three streams simultaneously. Can't predict which order the output will really be
    // in, though
    let stdout_chunks = vec![
        b"hel".to_vec(),
        b"lo ".to_vec(),
        b"wor".to_vec(),
        b"ld".to_vec(),
    ];

    let stderr_chunks = vec![b"standard ".to_vec(), b"error\n".to_vec()];

    let exit_code_values = vec![
        // There probably wouldn't be more than one exit code normally, but try just in case...
        Value::test_int(0),
        Value::test_int(1),
        Value::test_int(2),
    ];

    let stdout_len = stdout_chunks.iter().map(|c| c.len()).sum::<usize>() as u64;

    let stdout_raw_stream = RawStream::new(
        Box::new(stdout_chunks.clone().into_iter().map(Ok)),
        None,
        Span::test_data(),
        Some(stdout_len),
    );
    let stderr_raw_stream = RawStream::new(
        Box::new(stderr_chunks.clone().into_iter().map(Ok)),
        None,
        Span::test_data(),
        None,
    );
    let exit_code_stream = ListStream::from_stream(exit_code_values.clone().into_iter(), None);

    let data = PipelineData::ExternalStream {
        stdout: Some(stdout_raw_stream),
        stderr: Some(stderr_raw_stream),
        exit_code: Some(exit_code_stream),
        span: Span::test_data(),
        metadata: None,
        trim_end_newline: true,
    };

    test.engine_interface()
        .write_pipeline_data_response(data)
        .expect("failed to write external stream pipeline data");

    // First, there should be a header telling us metadata about the external stream
    match test.next_written_output() {
        Some(PluginOutput::CallResponse(PluginCallResponse::ExternalStream(info))) => {
            assert_eq!(Span::test_data(), info.span, "info.span");
            match info.stdout {
                Some(RawStreamInfo {
                    is_binary,
                    known_size,
                }) => {
                    let _ = is_binary; // undefined, could be anything
                    assert_eq!(Some(stdout_len), known_size);
                }
                None => todo!(),
            }
            match info.stderr {
                Some(RawStreamInfo {
                    is_binary,
                    known_size,
                }) => {
                    let _ = is_binary; // undefined, could be anything
                    assert_eq!(None, known_size);
                }
                None => todo!(),
            }
            assert!(info.has_exit_code);
            assert!(info.trim_end_newline);
        }
        Some(other) => panic!("unexpected response written: {other:?}"),
        None => panic!("no response written"),
    }

    // Then comes the hard part: check for the StreamData responses matching each of the iterators
    //
    // Each stream should be in order, but the order of the responses of unrelated streams is
    // not defined and may be random
    let mut stdout_iter = stdout_chunks.into_iter();
    let mut stderr_iter = stderr_chunks.into_iter();
    let mut exit_code_iter = exit_code_values.into_iter();

    for output in test.written_outputs() {
        match output {
            PluginOutput::StreamData(data) => match data {
                StreamData::List(_) => panic!("got unexpected list stream data: {data:?}"),
                StreamData::ExternalStdout(option) => {
                    let received = option
                        .transpose()
                        .expect("unexpected error in stdout stream");
                    assert_eq!(stdout_iter.next(), received);
                }
                StreamData::ExternalStderr(option) => {
                    let received = option
                        .transpose()
                        .expect("unexpected error in stderr stream");
                    assert_eq!(stderr_iter.next(), received);
                }
                StreamData::ExternalExitCode(received) => {
                    assert_eq!(exit_code_iter.next(), received);
                }
            },
            other => panic!("unexpected output: {other:?}"),
        }
    }

    // Make sure we got all of the messages we expected, and nothing extra
    assert!(
        stdout_iter.next().is_none(),
        "didn't match all stdout messages"
    );
    assert!(
        stderr_iter.next().is_none(),
        "didn't match all stderr messages"
    );
    assert!(
        exit_code_iter.next().is_none(),
        "didn't match all exit code messages"
    );

    assert!(!test.has_unconsumed_write());
}
