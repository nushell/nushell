use std::sync::Arc;

use crate::{plugin::interface::stream_data_io::StreamBuffers, StreamData};
use nu_protocol::Value;

macro_rules! gen_stream_data_tests {
    (
        $read_type:ident ($add_read:ident),
        $write_type:ident ($get_write:ident),
        |$test:ident| $gen_interface_impl:expr
    ) => {
        #[test]
        fn read_list_matches_input() {
            let $test = TestCase::new();
            $test.$add_read($read_type::StreamData(StreamData::List(Some(
                Value::test_bool(true),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::List(None)));

            let interface = $gen_interface_impl;
            interface.read.lock().unwrap().1 = StreamBuffers::new_list();

            match interface.read_list().unwrap() {
                Some(value) => assert_eq!(value, Value::test_bool(true)),
                None => panic!("expected to read list value, got end of list"),
            }

            match interface.read_list().unwrap() {
                Some(value) => panic!("expected to read end of list, got {value:?}"),
                None => (),
            }

            interface
                .read_list()
                .expect_err("didn't err on end of input");
        }

        #[test]
        fn read_external_matches_input() {
            let $test = TestCase::new();
            $test.$add_read($read_type::StreamData(StreamData::ExternalStdout(Some(
                Ok(vec![67]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![68]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalExitCode(Some(
                Value::test_int(1),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalExitCode(None)));

            let interface = $gen_interface_impl;
            interface.read.lock().unwrap().1 = StreamBuffers::new_external(true, true, true);

            match interface
                .read_external_stdout()
                .expect("while reading stdout")
            {
                Some(buffer) => assert_eq!(buffer, vec![67]),
                None => panic!("unexpected end of stdout stream"),
            }

            match interface
                .read_external_stderr()
                .expect("while reading stderr")
            {
                Some(buffer) => assert_eq!(buffer, vec![68]),
                None => panic!("unexpected end of stderr stream"),
            }

            match interface
                .read_external_exit_code()
                .expect("while reading exit code")
            {
                Some(value) => assert_eq!(value, Value::test_int(1)),
                None => panic!("unexpected end of exit code stream"),
            }

            match interface
                .read_external_exit_code()
                .expect("while reading exit code")
            {
                Some(value) => {
                    panic!("unexpected value in exit code stream, expected end: {value:?}")
                }
                None => (),
            }

            interface
                .read_external_exit_code()
                .expect_err("no error at end of input");
        }

        #[test]
        fn read_external_streams_out_of_input_order() {
            let $test = TestCase::new();
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![43]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalExitCode(Some(
                Value::test_int(42),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStdout(Some(
                Ok(vec![70]),
            ))));

            let interface = $gen_interface_impl;
            interface.read.lock().unwrap().1 = StreamBuffers::new_external(true, true, true);

            match interface
                .read_external_stdout()
                .expect("while reading stdout")
            {
                Some(buffer) => assert_eq!(buffer, vec![70]),
                None => panic!("unexpected end of stdout stream"),
            }

            match interface
                .read_external_stderr()
                .expect("while reading stderr")
            {
                Some(buffer) => assert_eq!(buffer, vec![43]),
                None => panic!("unexpected end of stderr stream"),
            }

            match interface
                .read_external_exit_code()
                .expect("while reading exit code")
            {
                Some(value) => assert_eq!(value, Value::test_int(42)),
                None => panic!("unexpected end of exit code stream"),
            }
        }

        #[test]
        fn read_external_streams_skip_dropped_stdout() {
            let $test = TestCase::new();
            $test.$add_read($read_type::StreamData(StreamData::ExternalStdout(Some(
                Ok(vec![1]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStdout(Some(
                Ok(vec![2]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStdout(Some(
                Ok(vec![3]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![42]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStdout(Some(
                Ok(vec![4]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![43]),
            ))));

            let interface = $gen_interface_impl;
            interface.read.lock().unwrap().1 = StreamBuffers::new_external(true, true, true);

            interface.drop_external_stdout();
            interface
                .read_external_stdout()
                .expect_err("reading from dropped stream should be err");
            assert_eq!(interface.read_external_stderr().unwrap(), Some(vec![42]));
            assert_eq!(interface.read_external_stderr().unwrap(), Some(vec![43]));
            assert!(interface
                .read
                .lock()
                .unwrap()
                .1
                .external_stdout
                .is_dropped());
            interface
                .read_external_stdout()
                .expect_err("reading from dropped stream should be err");
        }

        #[test]
        fn read_external_streams_skip_dropped_stderr() {
            let $test = TestCase::new();
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![1]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![2]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![3]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStdout(Some(
                Ok(vec![42]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![4]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStdout(Some(
                Ok(vec![43]),
            ))));

            let interface = $gen_interface_impl;
            interface.read.lock().unwrap().1 = StreamBuffers::new_external(true, true, true);

            interface.drop_external_stderr();
            interface
                .read_external_stderr()
                .expect_err("reading from dropped stream should be err");
            assert_eq!(interface.read_external_stdout().unwrap(), Some(vec![42]));
            assert_eq!(interface.read_external_stdout().unwrap(), Some(vec![43]));
            assert!(interface
                .read
                .lock()
                .unwrap()
                .1
                .external_stderr
                .is_dropped());
            interface
                .read_external_stderr()
                .expect_err("reading from dropped stream should be err");
        }

        #[test]
        fn read_external_streams_skip_dropped_exit_code() {
            let $test = TestCase::new();
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![2]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalExitCode(Some(
                Value::test_int(1),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStderr(Some(
                Ok(vec![3]),
            ))));
            $test.$add_read($read_type::StreamData(StreamData::ExternalStdout(Some(
                Ok(vec![42]),
            ))));

            let interface = $gen_interface_impl;
            interface.read.lock().unwrap().1 = StreamBuffers::new_external(true, true, true);

            interface.drop_external_exit_code();
            interface
                .read_external_exit_code()
                .expect_err("reading from dropped stream should be err");
            assert_eq!(interface.read_external_stderr().unwrap(), Some(vec![2]));
            assert_eq!(interface.read_external_stderr().unwrap(), Some(vec![3]));
            assert_eq!(interface.read_external_stdout().unwrap(), Some(vec![42]));
            assert!(interface
                .read
                .lock()
                .unwrap()
                .1
                .external_exit_code
                .is_dropped());
            interface
                .read_external_exit_code()
                .expect_err("reading from dropped stream should be err");
        }

        #[test]
        fn read_error_passthrough() {
            let $test = TestCase::new();
            let test_msg = "test io error";
            $test.set_read_error(ShellError::IOError {
                msg: test_msg.into(),
            });

            let interface = $gen_interface_impl;
            interface.read.lock().unwrap().1 = StreamBuffers::new_external(true, true, true);

            match interface
                .read_external_exit_code()
                .expect_err("succeeded unexpectedly")
            {
                ShellError::IOError { msg } => assert_eq!(test_msg, msg),
                other => panic!("other error: {other}"),
            }
        }

        #[test]
        fn write_error_passthrough() {
            let $test = TestCase::new();
            let test_msg = "test io error";
            $test.set_write_error(ShellError::IOError {
                msg: test_msg.into(),
            });

            let interface = $gen_interface_impl;

            match interface
                .write_list(None)
                .expect_err("succeeded unexpectedly")
            {
                ShellError::IOError { msg } => assert_eq!(test_msg, msg),
                other => panic!("other error: {other}"),
            }
            assert!(!$test.has_unconsumed_write());
        }

        #[test]
        fn write_list() {
            let $test = TestCase::new();
            let data = [Some(Value::test_int(1)), Some(Value::test_int(2)), None];
            let interface = $gen_interface_impl;
            for item in data.iter() {
                interface.write_list(item.clone()).expect("write failed");
            }
            for item in data.iter() {
                match $test.$get_write() {
                    Some($write_type::StreamData(StreamData::List(read_item))) => {
                        assert_eq!(item, &read_item)
                    }
                    Some(other) => panic!("got other data: {other:?}"),
                    None => panic!("no data was written for {item:?}"),
                }
            }
            assert!(!$test.has_unconsumed_write());
        }

        #[test]
        fn write_external_stdout() {
            let $test = TestCase::new();
            let data = [
                Some(Ok(vec![42])),
                Some(Ok(vec![80, 40])),
                Some(Err(ShellError::IOError {
                    msg: "test io error".into(),
                })),
                None,
            ];
            let interface = $gen_interface_impl;
            for item in data.iter() {
                interface
                    .write_external_stdout(item.clone())
                    .expect("write failed");
            }
            for item in data.iter() {
                match $test.$get_write() {
                    Some($write_type::StreamData(StreamData::ExternalStdout(read_item))) => {
                        match (item, &read_item) {
                            (Some(Ok(a)), Some(Ok(b))) => assert_eq!(a, b),
                            (Some(Err(a)), Some(Err(b))) => {
                                assert_eq!(a.to_string(), b.to_string())
                            }
                            (None, None) => (),
                            _ => panic!("expected {item:?}, got {read_item:?}"),
                        }
                    }
                    Some(other) => panic!("got other data: {other:?}"),
                    None => panic!("no data was written for {item:?}"),
                }
            }
            assert!(!$test.has_unconsumed_write());
        }

        #[test]
        fn write_external_stderr() {
            let $test = TestCase::new();
            let data = [
                Some(Ok(vec![42])),
                Some(Ok(vec![80, 40])),
                Some(Err(ShellError::IOError {
                    msg: "test io error".into(),
                })),
                None,
            ];
            let interface = $gen_interface_impl;
            for item in data.iter() {
                interface
                    .write_external_stderr(item.clone())
                    .expect("write failed");
            }
            for item in data.iter() {
                match $test.$get_write() {
                    Some($write_type::StreamData(StreamData::ExternalStderr(read_item))) => {
                        match (item, &read_item) {
                            (Some(Ok(a)), Some(Ok(b))) => assert_eq!(a, b),
                            (Some(Err(a)), Some(Err(b))) => {
                                assert_eq!(a.to_string(), b.to_string())
                            }
                            (None, None) => (),
                            _ => panic!("expected {item:?}, got {read_item:?}"),
                        }
                    }
                    Some(other) => panic!("got other data: {other:?}"),
                    None => panic!("no data was written for {item:?}"),
                }
            }
            assert!(!$test.has_unconsumed_write());
        }

        #[test]
        fn write_external_exit_code() {
            let $test = TestCase::new();
            let data = [Some(Value::test_int(1)), Some(Value::test_int(2)), None];
            let interface = $gen_interface_impl;
            for item in data.iter() {
                interface
                    .write_external_exit_code(item.clone())
                    .expect("write failed");
            }
            for item in data.iter() {
                match $test.$get_write() {
                    Some($write_type::StreamData(StreamData::ExternalExitCode(read_item))) => {
                        assert_eq!(item, &read_item)
                    }
                    Some(other) => panic!("got other data: {other:?}"),
                    None => panic!("no data was written for {item:?}"),
                }
            }
            assert!(!$test.has_unconsumed_write());
        }
    };
}

pub(crate) use gen_stream_data_tests;

use super::StreamBuffer;

#[test]
fn stream_buffers_default_doesnt_accept_stream_data() {
    let mut buffers = StreamBuffers::default();

    buffers
        .skip(StreamData::List(Some(Value::test_bool(true))))
        .expect_err("list was accepted");

    buffers
        .skip(StreamData::ExternalStdout(Some(Ok(vec![]))))
        .expect_err("external stdout was accepted");

    buffers
        .skip(StreamData::ExternalStderr(Some(Ok(vec![]))))
        .expect_err("external stderr was accepted");

    buffers
        .skip(StreamData::ExternalExitCode(Some(Value::test_int(1))))
        .expect_err("external exit code was accepted");
}

#[test]
fn stream_buffers_list_accepts_only_list_stream_data() {
    let mut buffers = StreamBuffers::new_list();

    buffers
        .skip(StreamData::List(Some(Value::test_bool(true))))
        .expect("list was not accepted");

    buffers
        .skip(StreamData::ExternalStdout(Some(Ok(vec![]))))
        .expect_err("external stdout was accepted");

    buffers
        .skip(StreamData::ExternalStderr(Some(Ok(vec![]))))
        .expect_err("external stderr was accepted");

    buffers
        .skip(StreamData::ExternalExitCode(Some(Value::test_int(1))))
        .expect_err("external exit code was accepted");
}

#[test]
fn stream_buffers_external_stream_stdout_accepts_only_external_stream_stdout_data() {
    let mut buffers = StreamBuffers::new_external(true, false, false);

    buffers
        .skip(StreamData::List(Some(Value::test_bool(true))))
        .expect_err("list was accepted");

    buffers
        .skip(StreamData::ExternalStdout(Some(Ok(vec![]))))
        .expect("external stdout was not accepted");

    buffers
        .skip(StreamData::ExternalStderr(Some(Ok(vec![]))))
        .expect_err("external stderr was accepted");

    buffers
        .skip(StreamData::ExternalExitCode(Some(Value::test_int(1))))
        .expect_err("external exit code was accepted");
}

#[test]
fn stream_buffers_external_stream_stderr_accepts_only_external_stream_stderr_data() {
    let mut buffers = StreamBuffers::new_external(false, true, false);

    buffers
        .skip(StreamData::List(Some(Value::test_bool(true))))
        .expect_err("list was accepted");

    buffers
        .skip(StreamData::ExternalStdout(Some(Ok(vec![]))))
        .expect_err("external stdout was accepted");

    buffers
        .skip(StreamData::ExternalStderr(Some(Ok(vec![]))))
        .expect("external stderr was not accepted");

    buffers
        .skip(StreamData::ExternalExitCode(Some(Value::test_int(1))))
        .expect_err("external exit code was accepted");
}

#[test]
fn stream_buffers_external_stream_exit_code_accepts_only_external_stream_exit_code_data() {
    let mut buffers = StreamBuffers::new_external(false, false, true);

    buffers
        .skip(StreamData::List(Some(Value::test_bool(true))))
        .expect_err("list was accepted");

    buffers
        .skip(StreamData::ExternalStdout(Some(Ok(vec![]))))
        .expect_err("external stdout was accepted");

    buffers
        .skip(StreamData::ExternalStderr(Some(Ok(vec![]))))
        .expect_err("external stderr was accepted");

    buffers
        .skip(StreamData::ExternalExitCode(Some(Value::test_int(1))))
        .expect("external exit code was not accepted");
}

#[test]
fn stream_buffers_external_stream_all_true_accepts_only_all_external_stream_data() {
    let mut buffers = StreamBuffers::new_external(true, true, true);

    buffers
        .skip(StreamData::List(Some(Value::test_bool(true))))
        .expect_err("list was accepted");

    buffers
        .skip(StreamData::ExternalStdout(Some(Ok(vec![]))))
        .expect("external stdout was not accepted");

    buffers
        .skip(StreamData::ExternalStderr(Some(Ok(vec![]))))
        .expect("external stderr was not accepted");

    buffers
        .skip(StreamData::ExternalExitCode(Some(Value::test_int(1))))
        .expect("external exit code was not accepted");
}

#[test]
fn stream_buffer_push_pop() {
    let mut buffer = StreamBuffer::Present(Default::default());
    buffer.push_back(1).unwrap();
    buffer.push_back(2).unwrap();
    assert_eq!(buffer.pop_front().unwrap(), Some(1));
    assert_eq!(buffer.pop_front().unwrap(), Some(2));
    assert_eq!(buffer.pop_front().unwrap(), None);
    buffer.push_back(42).unwrap();
    assert_eq!(buffer.pop_front().unwrap(), Some(42));
    assert_eq!(buffer.pop_front().unwrap(), None);
}

#[test]
fn stream_buffer_not_present_push_err() {
    StreamBuffer::NotPresent
        .push_back(2)
        .expect_err("should be an error");
}

#[test]
fn stream_buffer_not_present_pop_err() {
    StreamBuffer::<()>::NotPresent
        .pop_front()
        .expect_err("should be an error");
}

#[test]
fn stream_buffer_dropped_push() {
    // Use an Arc and a Weak copy of it as an indicator of whether the data is still alive
    let data = Arc::new(1);
    let data_weak = Arc::downgrade(&data);
    let mut dropped = StreamBuffer::Dropped;
    dropped.push_back(data).expect("can't push on dropped");
    // Should still be dropped - i.e., the message is not stored
    assert!(matches!(dropped, StreamBuffer::Dropped));
    // The data itself should also have been dropped - i.e., there are no copies of it around
    assert!(data_weak.upgrade().is_none(), "dropped data was preserved");
}

#[test]
fn stream_buffer_dropped_pop_err() {
    StreamBuffer::<()>::Dropped
        .pop_front()
        .expect_err("should be an error");
}
