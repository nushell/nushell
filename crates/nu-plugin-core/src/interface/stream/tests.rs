use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
        mpsc,
    },
    time::Duration,
};

use nu_utils::time::Instant;

use super::{
    ReaderMessage, StreamManager, StreamReader, StreamReaderSignal, StreamWriter,
    StreamWriterSignal, WriteStreamMessage,
};
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

/// Reports the empty-queue receive path, optionally pausing before the blocking receive.
struct ReceiveProbe {
    messages: mpsc::Sender<StreamMessage>,
    waiting: mpsc::Sender<()>,
    resume: Option<mpsc::Receiver<()>>,
}

impl WriteStreamMessage for ReceiveProbe {
    fn write_stream_message(&mut self, msg: StreamMessage) -> Result<(), ShellError> {
        self.messages.write_stream_message(msg)
    }

    fn flush(&mut self) -> Result<(), ShellError> {
        let _ = self.waiting.send(());
        if let Some(resume) = self.resume.take() {
            resume.recv().expect("receive probe was not released");
        }
        Ok(())
    }
}

const RECEIVE_TIMEOUT: Duration = Duration::from_secs(5);

fn test_reader<T, W>(id: usize, writer: W) -> (Arc<mpsc::Sender<ReaderMessage>>, StreamReader<T, W>)
where
    T: TryFrom<StreamData, Error = ShellError>,
    W: WriteStreamMessage,
{
    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(tx);
    let cancellation = StreamReaderSignal::new(&tx);
    let reader = StreamReader::new(id, rx, writer, cancellation);
    (tx, reader)
}

#[test]
fn reader_waits_for_peer_on_open_stream() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let (messages, _messages_rx) = mpsc::channel();
    let (waiting, waiting_rx) = mpsc::channel();
    let mut reader = manager.get_handle().read_stream::<Value, _>(
        0,
        ReceiveProbe {
            messages,
            waiting,
            resume: None,
        },
    )?;
    let (done, done_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = done.send(reader.recv());
    });
    waiting_rx
        .recv_timeout(RECEIVE_TIMEOUT)
        .expect("reader did not reach the empty-queue receive path");
    let pending = done_rx.recv_timeout(Duration::from_millis(50));
    // Always release the reader before asserting, including on a failed reproduction.
    manager.handle_message(StreamMessage::End(0))?;
    worker.join().expect("reader panicked");
    assert!(matches!(pending, Err(mpsc::RecvTimeoutError::Timeout)));
    assert!(
        done_rx
            .try_recv()
            .expect("reader did not finish")?
            .is_none()
    );
    Ok(())
}

#[test]
fn reader_cancellation_wakes_idle_input_and_check_to_wait_race() -> Result<(), ShellError> {
    for pause_before_wait in [false, true] {
        let manager = StreamManager::new();
        let (messages, messages_rx) = mpsc::channel();
        let (waiting, waiting_rx) = mpsc::channel();
        let (resume, resume_rx) = mpsc::channel();
        let mut reader = manager.get_handle().read_stream::<Value, _>(
            0,
            ReceiveProbe {
                messages,
                waiting,
                resume: pause_before_wait.then_some(resume_rx),
            },
        )?;
        let cancellation = reader.cancellation();
        let (done, done_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _ = done.send((reader.recv(), reader.recv()));
        });
        waiting_rx
            .recv_timeout(RECEIVE_TIMEOUT)
            .expect("reader did not reach receive path");
        cancellation.cancel();
        let _ = resume.send(());
        let result = done_rx.recv_timeout(RECEIVE_TIMEOUT);
        if result.is_err() {
            manager.handle_message(StreamMessage::End(0))?;
        }
        worker.join().expect("reader panicked");
        let (first, second) = result.expect("cancellation did not wake the reader");
        assert!(first?.is_none());
        assert!(second?.is_none());
        assert!(matches!(messages_rx.try_recv(), Ok(StreamMessage::Drop(0))));
        assert!(
            messages_rx.try_recv().is_err(),
            "duplicate Drop or unexpected Ack"
        );
        // No Data or End was needed: the producer's registration is still alive.
        assert!(manager.lock()?.reading_streams.contains_key(&0));
    }
    Ok(())
}

#[test]
fn reader_cancellation_discards_buffered_values_before_and_during_receive() -> Result<(), ShellError>
{
    for cancel_after_check in [false, true] {
        let manager = StreamManager::new();
        let (messages, messages_rx) = mpsc::channel();
        let mut reader = manager.get_handle().read_stream::<Value, _>(0, messages)?;
        let cancellation = reader.cancellation();
        let mut buffers = Vec::new();
        for n in 0..4 {
            let values = Arc::new(vec![Value::test_int(n)]);
            buffers.push(Arc::downgrade(&values));
            let value = Value::list_shared(values.into(), nu_protocol::Span::test_data());
            manager.handle_message(StreamMessage::Data(0, value.into()))?;
        }
        let (entered, entered_rx) = mpsc::channel();
        let (resume, resume_rx) = mpsc::channel();
        if cancel_after_check {
            reader.before_receive = Some((entered, resume_rx));
        } else {
            cancellation.cancel();
        }
        let (done, done_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let first = reader.next();
            let second = reader.next();
            let _ = done.send((first, second, reader.receiver.is_none()));
        });
        if cancel_after_check {
            let entered = entered_rx.recv_timeout(RECEIVE_TIMEOUT);
            cancellation.cancel();
            let _ = resume.send(());
            entered.expect("reader did not reach receive hook");
        }
        let result = done_rx.recv_timeout(RECEIVE_TIMEOUT);
        if result.is_err() {
            manager.handle_message(StreamMessage::End(0))?;
        }
        worker.join().expect("reader panicked");
        let (first, second, closed) = result.expect("reader did not stop");
        assert!(first.is_none() && second.is_none() && closed);
        assert!(buffers.iter().all(|buffer| buffer.upgrade().is_none()));
        assert!(matches!(messages_rx.try_recv(), Ok(StreamMessage::Drop(0))));
        assert!(
            messages_rx.try_recv().is_err(),
            "cancelled data must not be acknowledged"
        );
    }
    Ok(())
}

#[test]
fn reader_disconnection_with_live_cancellation_handles() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let (messages, _messages_rx) = mpsc::channel();
    let (waiting, waiting_rx) = mpsc::channel();
    let mut reader = manager.get_handle().read_stream::<Value, _>(
        0,
        ReceiveProbe {
            messages,
            waiting,
            resume: None,
        },
    )?;
    let cancellation = reader.cancellation();
    let other_handle = cancellation.clone();
    let (done, done_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = done.send(reader.recv());
    });
    waiting_rx
        .recv_timeout(RECEIVE_TIMEOUT)
        .expect("reader did not reach receive path");
    drop(manager);
    let result = done_rx.recv_timeout(RECEIVE_TIMEOUT);
    if result.is_err() {
        cancellation.cancel();
    }
    worker.join().expect("reader panicked");
    let error = result
        .expect("handle kept the channel connected")
        .expect_err("expected disconnection");
    assert!(format!("{error:?}").contains("connection lost"));
    assert!(other_handle.sender.upgrade().is_none());
    Ok(())
}

#[test]
fn reader_cancellation_is_idempotent_and_safe_after_eof() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let (messages, messages_rx) = mpsc::channel();
    let mut reader = manager.get_handle().read_stream::<Value, _>(0, messages)?;
    let cancellation = reader.cancellation();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let cancellation = cancellation.clone();
            scope.spawn(move || cancellation.cancel());
        }
    });
    assert!(reader.recv()?.is_none());
    cancellation.cancel();
    assert!(reader.recv()?.is_none());
    drop(reader);
    cancellation.cancel();
    assert!(matches!(messages_rx.try_recv(), Ok(StreamMessage::Drop(0))));
    assert!(messages_rx.try_recv().is_err());
    manager.handle_message(StreamMessage::End(0))?;

    let (messages, messages_rx) = mpsc::channel();
    let mut reader = manager.get_handle().read_stream::<Value, _>(1, messages)?;
    let cancellation = reader.cancellation();
    manager.handle_message(StreamMessage::Data(1, Value::test_int(1).into()))?;
    manager.handle_message(StreamMessage::End(1))?;
    assert_eq!(reader.recv()?, Some(Value::test_int(1)));
    assert!(reader.recv()?.is_none());
    cancellation.cancel();
    assert!(reader.recv()?.is_none());
    drop(reader);
    assert!(matches!(messages_rx.try_recv(), Ok(StreamMessage::Ack(1))));
    assert!(matches!(messages_rx.try_recv(), Ok(StreamMessage::Drop(1))));
    assert!(messages_rx.try_recv().is_err());
    Ok(())
}

#[test]
fn reader_cancellation_accepts_late_messages_but_rejects_unknown_streams() -> Result<(), ShellError>
{
    let manager = StreamManager::new();
    let mut reader = manager
        .get_handle()
        .read_stream::<Value, _>(0, TestSink::default())?;
    let cancellation = reader.cancellation();
    cancellation.cancel();
    assert!(reader.next().is_none());
    let values = Arc::new(vec![Value::test_int(42)]);
    let weak = Arc::downgrade(&values);
    manager.handle_message(StreamMessage::Data(
        0,
        Value::list_shared(values.into(), nu_protocol::Span::test_data()).into(),
    ))?;
    assert!(weak.upgrade().is_none(), "late values were buffered");
    manager.handle_message(StreamMessage::End(0))?;
    assert!(cancellation.sender.upgrade().is_none());
    assert!(!manager.lock()?.reading_streams.contains_key(&0));
    for message in [
        StreamMessage::Data(0, Value::test_int(0).into()),
        StreamMessage::Data(999, Value::test_int(0).into()),
        StreamMessage::End(999),
    ] {
        assert!(matches!(
            manager.handle_message(message),
            Err(ShellError::PluginFailedToDecode { .. })
        ));
    }
    Ok(())
}

#[test]
fn reader_old_cancellation_does_not_cancel_reused_stream_id() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let mut old = manager
        .get_handle()
        .read_stream::<Value, _>(0, TestSink::default())?;
    let cancellation = old.cancellation();
    manager.handle_message(StreamMessage::End(0))?;
    assert!(old.next().is_none());
    drop(old);
    let mut new = manager
        .get_handle()
        .read_stream::<Value, _>(0, TestSink::default())?;
    cancellation.cancel();
    manager.handle_message(StreamMessage::Data(0, Value::test_int(42).into()))?;
    manager.handle_message(StreamMessage::End(0))?;
    assert_eq!(new.next(), Some(Value::test_int(42)));
    assert!(new.next().is_none());
    Ok(())
}

#[test]
fn reader_cancellation_notification_error_closes_queue_once() -> Result<(), ShellError> {
    use std::sync::atomic::AtomicUsize;

    struct FailingSink {
        drops: Arc<AtomicUsize>,
        fail_flush: bool,
    }
    impl WriteStreamMessage for FailingSink {
        fn write_stream_message(&mut self, msg: StreamMessage) -> Result<(), ShellError> {
            assert!(matches!(msg, StreamMessage::Drop(0)));
            self.drops.fetch_add(1, Relaxed);
            if self.fail_flush {
                Ok(())
            } else {
                Err(ShellError::NushellFailed {
                    msg: "drop failure".into(),
                })
            }
        }
        fn flush(&mut self) -> Result<(), ShellError> {
            Err(ShellError::NushellFailed {
                msg: "flush failure".into(),
            })
        }
    }

    for fail_flush in [false, true] {
        let manager = StreamManager::new();
        let drops = Arc::new(AtomicUsize::new(0));
        let mut reader = manager.get_handle().read_stream::<Value, _>(
            0,
            FailingSink {
                drops: drops.clone(),
                fail_flush,
            },
        )?;
        let values = Arc::new(vec![Value::test_int(1)]);
        let weak = Arc::downgrade(&values);
        manager.handle_message(StreamMessage::Data(
            0,
            Value::list_shared(values.into(), nu_protocol::Span::test_data()).into(),
        ))?;
        reader.cancellation().cancel();
        let error = reader.recv().expect_err("notification should fail");
        assert!(error.to_string().contains(if fail_flush {
            "flush failure"
        } else {
            "drop failure"
        }));
        assert!(weak.upgrade().is_none());
        assert!(reader.receiver.is_none());
        assert!(reader.recv()?.is_none());
        drop(reader);
        assert_eq!(drops.load(Relaxed), 1);
    }
    Ok(())
}

#[test]
fn reader_cancellation_handles_do_not_retain_reader_or_cancel_on_drop() -> Result<(), ShellError> {
    use std::sync::atomic::AtomicUsize;

    struct DropSink(Arc<AtomicUsize>);
    impl Drop for DropSink {
        fn drop(&mut self) {
            self.0.fetch_add(1, Relaxed);
        }
    }
    impl WriteStreamMessage for DropSink {
        fn write_stream_message(&mut self, _: StreamMessage) -> Result<(), ShellError> {
            Ok(())
        }
        fn flush(&mut self) -> Result<(), ShellError> {
            Ok(())
        }
    }
    let manager = StreamManager::new();
    let drops = Arc::new(AtomicUsize::new(0));
    let mut reader = manager
        .get_handle()
        .read_stream::<Value, _>(0, DropSink(drops.clone()))?;
    drop(reader.cancellation());
    manager.handle_message(StreamMessage::Data(0, Value::test_int(42).into()))?;
    assert_eq!(reader.next(), Some(Value::test_int(42)));
    let cancellation = reader.cancellation();
    let values = Arc::new(vec![Value::test_int(1)]);
    let weak = Arc::downgrade(&values);
    manager.handle_message(StreamMessage::Data(
        0,
        Value::list_shared(values.into(), nu_protocol::Span::test_data()).into(),
    ))?;
    drop(reader);
    assert_eq!(drops.load(Relaxed), 1);
    assert!(weak.upgrade().is_none());
    cancellation.cancel();
    manager.handle_message(StreamMessage::End(0))?;
    assert!(cancellation.sender.upgrade().is_none());
    Ok(())
}

#[test]
fn reader_cancellation_races_data_end_and_disconnection() -> Result<(), ShellError> {
    for event in 0..3 {
        let manager = StreamManager::new();
        let (messages, messages_rx) = mpsc::channel();
        let mut reader = manager.get_handle().read_stream::<Value, _>(0, messages)?;
        let cancellation = reader.cancellation();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let cancel_barrier = barrier.clone();
        let cancel_worker = std::thread::spawn(move || {
            cancel_barrier.wait();
            cancellation.cancel();
        });
        let (done, done_rx) = mpsc::channel();
        let reader_worker = std::thread::spawn(move || {
            let values = reader.by_ref().collect::<Vec<_>>();
            let _ = done.send((values, reader.next()));
        });
        barrier.wait();
        if event == 0 {
            manager.handle_message(StreamMessage::Data(0, Value::test_int(42).into()))?;
        }
        if event != 2 {
            manager.handle_message(StreamMessage::End(0))?;
        }
        drop(manager);
        cancel_worker.join().expect("cancel panicked");
        let (values, next) = done_rx
            .recv_timeout(RECEIVE_TIMEOUT)
            .expect("reader did not stop");
        reader_worker.join().expect("reader panicked");
        assert!(values.len() <= 1);
        assert!(next.is_none());
        if event != 2 {
            assert!(values.iter().all(|value| !value.is_error()));
        }
        let messages = messages_rx.try_iter().collect::<Vec<_>>();
        assert_eq!(
            messages
                .iter()
                .filter(|msg| matches!(msg, StreamMessage::Drop(0)))
                .count(),
            1
        );
    }
    Ok(())
}

#[test]
fn reader_cancellation_preserves_error_received_after_initial_check() -> Result<(), ShellError> {
    let manager = StreamManager::new();
    let mut reader = manager
        .get_handle()
        .read_stream::<Value, _>(0, TestSink::default())?;
    let error = ShellError::PluginFailedToDecode {
        msg: "original error".into(),
    };
    manager.broadcast_read_error(error.clone())?;
    let (entered, entered_rx) = mpsc::channel();
    let (resume, resume_rx) = mpsc::channel();
    reader.before_receive = Some((entered, resume_rx));
    let cancellation = reader.cancellation();
    let (done, done_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = done.send((reader.recv(), reader.recv()));
    });
    let entered = entered_rx.recv_timeout(RECEIVE_TIMEOUT);
    cancellation.cancel();
    let _ = resume.send(());
    entered.expect("reader did not reach receive hook");
    let result = done_rx.recv_timeout(RECEIVE_TIMEOUT);
    if result.is_err() {
        manager.handle_message(StreamMessage::End(0))?;
    }
    worker.join().expect("reader panicked");
    let (received, next) = result.expect("reader did not stop");
    assert_eq!(
        received.expect_err("expected original error").to_string(),
        error.to_string()
    );
    assert!(next?.is_none());
    Ok(())
}

#[test]
fn reader_recv_list_messages() -> Result<(), ShellError> {
    let (tx, mut reader) = test_reader::<Value, _>(0, TestSink::default());

    tx.send(ReaderMessage::Incoming(Ok(Some(StreamData::List(
        Value::test_int(5),
    )))))
    .unwrap();
    drop(tx);

    assert_eq!(Some(Value::test_int(5)), reader.recv()?);
    Ok(())
}

#[test]
fn list_reader_recv_wrong_type() -> Result<(), ShellError> {
    let (tx, mut reader) = test_reader::<Value, _>(0, TestSink::default());

    tx.send(ReaderMessage::Incoming(Ok(Some(StreamData::Raw(Ok(
        vec![10, 20],
    ))))))
    .unwrap();
    tx.send(ReaderMessage::Incoming(Ok(Some(StreamData::List(
        Value::test_nothing(),
    )))))
    .unwrap();
    drop(tx);

    reader.recv().expect_err("should be an error");
    reader.recv().expect("should be able to recover");

    Ok(())
}

#[test]
fn reader_recv_raw_messages() -> Result<(), ShellError> {
    let (tx, mut reader) = test_reader::<Result<Vec<u8>, ShellError>, _>(0, TestSink::default());

    tx.send(ReaderMessage::Incoming(Ok(Some(StreamData::Raw(Ok(
        vec![10, 20],
    ))))))
    .unwrap();
    drop(tx);

    assert_eq!(Some(vec![10, 20]), reader.recv()?.transpose()?);
    Ok(())
}

#[test]
fn raw_reader_recv_wrong_type() -> Result<(), ShellError> {
    let (tx, mut reader) = test_reader::<Result<Vec<u8>, ShellError>, _>(0, TestSink::default());

    tx.send(ReaderMessage::Incoming(Ok(Some(StreamData::List(
        Value::test_nothing(),
    )))))
    .unwrap();
    tx.send(ReaderMessage::Incoming(Ok(Some(StreamData::Raw(Ok(
        vec![10, 20],
    ))))))
    .unwrap();
    drop(tx);

    reader.recv().expect_err("should be an error");
    reader.recv().expect("should be able to recover");

    Ok(())
}

#[test]
fn reader_recv_acknowledge() -> Result<(), ShellError> {
    let (tx, mut reader) = test_reader::<Value, _>(0, TestSink::default());

    tx.send(ReaderMessage::Incoming(Ok(Some(StreamData::List(
        Value::test_int(5),
    )))))
    .unwrap();
    tx.send(ReaderMessage::Incoming(Ok(Some(StreamData::List(
        Value::test_int(6),
    )))))
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
    let (tx, mut reader) = test_reader::<Value, _>(0, TestSink::default());

    tx.send(ReaderMessage::Incoming(Ok(Some(StreamData::List(
        Value::test_int(5),
    )))))
    .unwrap();
    tx.send(ReaderMessage::Incoming(Ok(None))).unwrap();
    drop(tx);

    assert!(reader.recv()?.is_some(), "actual message");
    assert!(reader.recv()?.is_none(), "on close");
    assert!(reader.recv()?.is_none(), "after close");
    Ok(())
}

#[test]
fn reader_iter_fuse_on_error() -> Result<(), ShellError> {
    let (tx, mut reader) = test_reader::<Value, _>(0, TestSink::default());

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

    let (_tx, reader) = test_reader::<Value, _>(1, Check(flag.clone()));
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
