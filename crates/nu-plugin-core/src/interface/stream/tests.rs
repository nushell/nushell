use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
        mpsc,
    },
    time::{Duration, Instant},
};

use super::{StreamManager, StreamReader, StreamWriter, StreamWriterSignal, WriteStreamMessage};
use nu_plugin_protocol::{StreamData, StreamMessage};
use nu_protocol::{ShellError, Value};

// Should be long enough to definitely complete any quick operation, but not so long that tests are
// slow to complete. 10 ms is a pretty long time
const WAIT_DURATION: Duration = Duration::from_millis(10);

// Maximum time to wait for a condition to be true
const MAX_WAIT_DURATION: Duration = Duration::from_millis(500);

/// Wait for a condition to be true, or panic if the duration exceeds MAX_WAIT_DURATION
#[track_caller]
fn wait_for_condition(mut cond: impl FnMut() -> bool, message: &str) {
    // Early check
    if cond() {
        return;
    }

    let start = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(10));

        if cond() {
            return;
        }

        let elapsed = Instant::now().saturating_duration_since(start);
        if elapsed > MAX_WAIT_DURATION {
            panic!(
                "{message}: Waited {:.2}sec, which is more than the maximum of {:.2}sec",
                elapsed.as_secs_f64(),
                MAX_WAIT_DURATION.as_secs_f64(),
            );
        }
    }
}

#[derive(Debug, Clone, Default)]
struct TestSink(Vec<StreamMessage>);

impl WriteStreamMessage for TestSink {
    fn write_stream_message(&mut self, msg: StreamMessage) -> Result<(), ShellError> {
        self.0.push(msg);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), ShellError> {
        Ok(())
    }
}

impl WriteStreamMessage for mpsc::Sender<StreamMessage> {
    fn write_stream_message(&mut self, msg: StreamMessage) -> Result<(), ShellError> {
        self.send(msg).map_err(|err| ShellError::NushellFailed {
            msg: err.to_string(),
        })
    }

    fn flush(&mut self) -> Result<(), ShellError> {
        Ok(())
    }
}

#[test]
fn reader_recv_list_messages() -> Result<(), ShellError> {
    let (tx, rx) = mpsc::channel();
    let mut reader = StreamReader::new(0, rx, TestSink::default());

    tx.send(Ok(Some(StreamData::List(Value::test_int(5)))))
        .unwrap();
    drop(tx);

    assert_eq!(Some(Value::test_int(5)), reader.recv()?);
    Ok(())
}

#[test]
fn list_reader_recv_wrong_type() -> Result<(), ShellError> {
    let (tx, rx) = mpsc::channel();
    let mut reader = StreamReader::<Value, _>::new(0, rx, TestSink::default());

    tx.send(Ok(Some(StreamData::Raw(Ok(vec![10, 20])))))
        .unwrap();
    tx.send(Ok(Some(StreamData::List(Value::test_nothing()))))
        .unwrap();
    drop(tx);

    reader.recv().expect_err("should be an error");
    reader.recv().expect("should be able to recover");

    Ok(())
}

#[test]
fn reader_recv_raw_messages() -> Result<(), ShellError> {
    let (tx, rx) = mpsc::channel();
    let mut reader =
        StreamReader::<Result<Vec<u8>, ShellError>, _>::new(0, rx, TestSink::default());

    tx.send(Ok(Some(StreamData::Raw(Ok(vec![10, 20])))))
        .unwrap();
    drop(tx);

    assert_eq!(Some(vec![10, 20]), reader.recv()?.transpose()?);
    Ok(())
}

#[test]
fn raw_reader_recv_wrong_type() -> Result<(), ShellError> {
    let (tx, rx) = mpsc::channel();
    let mut reader =
        StreamReader::<Result<Vec<u8>, ShellError>, _>::new(0, rx, TestSink::default());

    tx.send(Ok(Some(StreamData::List(Value::test_nothing()))))
        .unwrap();
    tx.send(Ok(Some(StreamData::Raw(Ok(vec![10, 20])))))
        .unwrap();
    drop(tx);

    reader.recv().expect_err("should be an error");
    reader.recv().expect("should be able to recover");

    Ok(())
}

#[test]
fn reader_recv_acknowledge() -> Result<(), ShellError> {
    let (tx, rx) = mpsc::channel();
    let mut reader = StreamReader::<Value, _>::new(0, rx, TestSink::default());

    tx.send(Ok(Some(StreamData::List(Value::test_int(5)))))
        .unwrap();
    tx.send(Ok(Some(StreamData::List(Value::test_int(6)))))
        .unwrap();
    drop(tx);

    reader.recv()?;
    reader.recv()?;
    let wrote = &reader.writer.0;
    assert!(wrote.len() >= 2);
    assert!(
        matches!(wrote[0], StreamMessage::Ack(0)),
        "0 = {:?}",
        wrote[0]
    );
    assert!(
        matches!(wrote[1], StreamMessage::Ack(0)),
        "1 = {:?}",
        wrote[1]
    );
    Ok(())
}

#[test]
fn reader_recv_end_of_stream() -> Result<(), ShellError> {
    let (tx, rx) = mpsc::channel();
    let mut reader = StreamReader::<Value, _>::new(0, rx, TestSink::default());

    tx.send(Ok(Some(StreamData::List(Value::test_int(5)))))
        .unwrap();
    tx.send(Ok(None)).unwrap();
    drop(tx);

    assert!(reader.recv()?.is_some(), "actual message");
    assert!(reader.recv()?.is_none(), "on close");
    assert!(reader.recv()?.is_none(), "after close");
    Ok(())
}

#[test]
fn reader_iter_fuse_on_error() -> Result<(), ShellError> {
    let (tx, rx) = mpsc::channel();
    let mut reader = StreamReader::<Value, _>::new(0, rx, TestSink::default());

    drop(tx); // should cause error, because we didn't explicitly signal the end

    assert!(
        reader.next().is_some_and(|e| e.is_error()),
        "should be error the first time"
    );
    assert!(reader.next().is_none(), "should be closed the second time");
    Ok(())
}

#[test]
fn reader_drop() {
    let (_tx, rx) = mpsc::channel();

    // Flag set if drop message is received.
    struct Check(Arc<AtomicBool>);

    impl WriteStreamMessage for Check {
        fn write_stream_message(&mut self, msg: StreamMessage) -> Result<(), ShellError> {
            assert!(matches!(msg, StreamMessage::Drop(1)), "got {msg:?}");
            self.0.store(true, Relaxed);
            Ok(())
        }

        fn flush(&mut self) -> Result<(), ShellError> {
            Ok(())
        }
    }

    let flag = Arc::new(AtomicBool::new(false));

    let reader = StreamReader::<Value, _>::new(1, rx, Check(flag.clone()));
    drop(reader);

    assert!(flag.load(Relaxed));
}

#[test]
fn writer_write_all_stops_if_dropped() -> Result<(), ShellError> {
    let signal = Arc::new(StreamWriterSignal::new(20));
    let id = 1337;
    let mut writer = StreamWriter::new(id, signal.clone(), TestSink::default());

    // Simulate this by having it consume a stream that will actually do the drop halfway through
    let iter = (0..5).map(Value::test_int).chain({
        let mut n = 5;
        std::iter::from_fn(move || {
            // produces numbers 5..10, but drops for the first one
            if n == 5 {
                signal.set_dropped().unwrap();
            }
            if n < 10 {
                let value = Value::test_int(n);
                n += 1;
                Some(value)
            } else {
                None
            }
        })
    });

    writer.write_all(iter)?;

    assert!(writer.is_dropped()?);

    let wrote = &writer.writer.0;
    assert_eq!(5, wrote.len(), "length wrong: {wrote:?}");

    for (n, message) in (0..5).zip(wrote) {
        match message {
            StreamMessage::Data(msg_id, StreamData::List(value)) => {
                assert_eq!(id, *msg_id, "id");
                assert_eq!(Value::test_int(n), *value, "value");
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    Ok(())
}

#[test]
fn writer_end() -> Result<(), ShellError> {
    let signal = Arc::new(StreamWriterSignal::new(20));
    let mut writer = StreamWriter::new(9001, signal.clone(), TestSink::default());

    writer.end()?;
    writer
        .write(Value::test_int(2))
        .expect_err("shouldn't be able to write after end");
    writer.end().expect("end twice should be ok");

    let wrote = &writer.writer.0;
    assert!(
        matches!(wrote.last(), Some(StreamMessage::End(9001))),
        "didn't write end message: {wrote:?}"
    );

    Ok(())
}

#[test]
fn signal_set_dropped() -> Result<(), ShellError> {
    let signal = StreamWriterSignal::new(4);
    assert!(!signal.is_dropped()?);
    signal.set_dropped()?;
    assert!(signal.is_dropped()?);
    Ok(())
}

#[test]
fn signal_notify_sent_false_if_unacknowledged() -> Result<(), ShellError> {
    let signal = StreamWriterSignal::new(2);
    assert!(signal.notify_sent()?);
    for _ in 0..100 {
        assert!(!signal.notify_sent()?);
    }
    Ok(())
}

#[test]
fn signal_notify_sent_never_false_if_flowing() -> Result<(), ShellError> {
    let signal = StreamWriterSignal::new(1);
    for _ in 0..100 {
        signal.notify_acknowledged()?;
    }
    for _ in 0..100 {
        assert!(signal.notify_sent()?);
    }
    Ok(())
}

#[test]
fn signal_wait_for_drain_blocks_on_unacknowledged() -> Result<(), ShellError> {
    let signal = StreamWriterSignal::new(50);
    std::thread::scope(|scope| {
        let spawned = scope.spawn(|| {
            for _ in 0..100 {
                if !signal.notify_sent()? {
                    signal.wait_for_drain()?;
                }
            }
            Ok(())
        });
        std::thread::sleep(WAIT_DURATION);
        assert!(!spawned.is_finished(), "didn't block");
        for _ in 0..100 {
            signal.notify_acknowledged()?;
        }
        wait_for_condition(|| spawned.is_finished(), "blocked at end");
        spawned.join().unwrap()
    })
}

#[test]
fn signal_wait_for_drain_unblocks_on_dropped() -> Result<(), ShellError> {
    let signal = StreamWriterSignal::new(1);
    std::thread::scope(|scope| {
        let spawned = scope.spawn(|| {
            while !signal.is_dropped()? {
                if !signal.notify_sent()? {
                    signal.wait_for_drain()?;
                }
            }
            Ok(())
        });
        std::thread::sleep(WAIT_DURATION);
        assert!(!spawned.is_finished(), "didn't block");
        signal.set_dropped()?;
        wait_for_condition(|| spawned.is_finished(), "still blocked at end");
        spawned.join().unwrap()
    })
}

#[test]
fn stream_manager_single_stream_read_scenario() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let handle = manager.get_handle();
    let (tx, rx) = mpsc::channel();
    let readable = handle.read_stream::<Value, _>(2, tx)?;

    let expected_values = vec![Value::test_int(40), Value::test_string("hello")];

    for value in &expected_values {
        manager.handle_message(StreamMessage::Data(2, value.clone().into()))?;
    }
    manager.handle_message(StreamMessage::End(2))?;

    let values = readable.collect::<Vec<Value>>();

    assert_eq!(expected_values, values);

    // Now check the sent messages on consumption
    // Should be Ack for each message, then Drop
    for _ in &expected_values {
        match rx.try_recv().expect("failed to receive Ack") {
            StreamMessage::Ack(2) => (),
            other => panic!("should have been an Ack: {other:?}"),
        }
    }
    match rx.try_recv().expect("failed to receive Drop") {
        StreamMessage::Drop(2) => (),
        other => panic!("should have been a Drop: {other:?}"),
    }

    Ok(())
}

#[test]
fn stream_manager_multi_stream_read_scenario() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let handle = manager.get_handle();
    let (tx, rx) = mpsc::channel();
    let readable_list = handle.read_stream::<Value, _>(2, tx.clone())?;
    let readable_raw = handle.read_stream::<Result<Vec<u8>, _>, _>(3, tx)?;

    let expected_values = (1..100).map(Value::test_int).collect::<Vec<_>>();
    let expected_raw_buffers = (1..100).map(|n| vec![n]).collect::<Vec<Vec<u8>>>();

    for (value, buf) in expected_values.iter().zip(&expected_raw_buffers) {
        manager.handle_message(StreamMessage::Data(2, value.clone().into()))?;
        manager.handle_message(StreamMessage::Data(3, StreamData::Raw(Ok(buf.clone()))))?;
    }
    manager.handle_message(StreamMessage::End(2))?;
    manager.handle_message(StreamMessage::End(3))?;

    let values = readable_list.collect::<Vec<Value>>();
    let bufs = readable_raw.collect::<Result<Vec<Vec<u8>>, _>>()?;

    for (expected_value, value) in expected_values.iter().zip(&values) {
        assert_eq!(expected_value, value, "in List stream");
    }
    for (expected_buf, buf) in expected_raw_buffers.iter().zip(&bufs) {
        assert_eq!(expected_buf, buf, "in Raw stream");
    }

    // Now check the sent messages on consumption
    // Should be Ack for each message, then Drop
    for _ in &expected_values {
        match rx.try_recv().expect("failed to receive Ack") {
            StreamMessage::Ack(2) => (),
            other => panic!("should have been an Ack(2): {other:?}"),
        }
    }
    match rx.try_recv().expect("failed to receive Drop") {
        StreamMessage::Drop(2) => (),
        other => panic!("should have been a Drop(2): {other:?}"),
    }
    for _ in &expected_values {
        match rx.try_recv().expect("failed to receive Ack") {
            StreamMessage::Ack(3) => (),
            other => panic!("should have been an Ack(3): {other:?}"),
        }
    }
    match rx.try_recv().expect("failed to receive Drop") {
        StreamMessage::Drop(3) => (),
        other => panic!("should have been a Drop(3): {other:?}"),
    }

    // Should be end of stream
    assert!(
        rx.try_recv().is_err(),
        "more messages written to stream than expected"
    );

    Ok(())
}

#[test]
fn stream_manager_write_scenario() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let handle = manager.get_handle();
    let (tx, rx) = mpsc::channel();
    let mut writable = handle.write_stream(4, tx, 100)?;

    let expected_values = vec![b"hello".to_vec(), b"world".to_vec(), b"test".to_vec()];

    for value in &expected_values {
        writable.write(Ok::<_, ShellError>(value.clone()))?;
    }

    // Now try signalling ack
    assert_eq!(
        expected_values.len() as i32,
        writable.signal.lock()?.unacknowledged,
        "unacknowledged initial count",
    );
    manager.handle_message(StreamMessage::Ack(4))?;
    assert_eq!(
        expected_values.len() as i32 - 1,
        writable.signal.lock()?.unacknowledged,
        "unacknowledged post-Ack count",
    );

    // ...and Drop
    manager.handle_message(StreamMessage::Drop(4))?;
    assert!(writable.is_dropped()?);

    // Drop the StreamWriter...
    drop(writable);

    // now check what was actually written
    for value in &expected_values {
        match rx.try_recv().expect("failed to receive Data") {
            StreamMessage::Data(4, StreamData::Raw(Ok(received))) => {
                assert_eq!(*value, received);
            }
            other @ StreamMessage::Data(..) => panic!("wrong Data for {value:?}: {other:?}"),
            other => panic!("should have been Data: {other:?}"),
        }
    }
    match rx.try_recv().expect("failed to receive End") {
        StreamMessage::End(4) => (),
        other => panic!("should have been End: {other:?}"),
    }

    Ok(())
}

#[test]
fn stream_manager_broadcast_read_error() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let handle = manager.get_handle();
    let mut readable0 = handle.read_stream::<Value, _>(0, TestSink::default())?;
    let mut readable1 = handle.read_stream::<Result<Vec<u8>, _>, _>(1, TestSink::default())?;

    let error = ShellError::PluginFailedToDecode {
        msg: "test decode error".into(),
    };

    manager.broadcast_read_error(error.clone())?;
    drop(manager);

    assert_eq!(
        error.to_string(),
        readable0
            .recv()
            .transpose()
            .expect("nothing received from readable0")
            .expect_err("not an error received from readable0")
            .to_string()
    );
    assert_eq!(
        error.to_string(),
        readable1
            .next()
            .expect("nothing received from readable1")
            .expect_err("not an error received from readable1")
            .to_string()
    );
    Ok(())
}

#[test]
fn stream_manager_drop_writers_on_drop() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let handle = manager.get_handle();
    let writable = handle.write_stream(4, TestSink::default(), 100)?;

    assert!(!writable.is_dropped()?);

    drop(manager);

    assert!(writable.is_dropped()?);

    Ok(())
}
