use futures::stream::Stream;
use std::pin::Pin;
use std::sync::{mpsc, Arc, Mutex};
use std::task::{self, Poll, Waker};
use std::thread;

#[allow(clippy::option_option)]
struct SharedState<T: Send + 'static> {
    result: Option<Option<T>>,
    kill: bool,
    waker: Option<Waker>,
}

pub struct ThreadedReceiver<T: Send + 'static> {
    shared_state: Arc<Mutex<SharedState<T>>>,
}

impl<T: Send + 'static> ThreadedReceiver<T> {
    pub fn new(recv: mpsc::Receiver<T>) -> ThreadedReceiver<T> {
        let shared_state = Arc::new(Mutex::new(SharedState {
            result: None,
            kill: false,
            waker: None,
        }));

        // Clone everything to avoid lifetimes
        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            loop {
                let result = recv.recv();

                {
                    let mut shared_state = thread_shared_state
                        .lock()
                        .expect("ThreadedFuture shared state shouldn't be poisoned");

                    if let Ok(result) = result {
                        shared_state.result = Some(Some(result));
                    } else {
                        break;
                    }
                }

                // Don't attempt to recv anything else until consumed
                loop {
                    let mut shared_state = thread_shared_state
                        .lock()
                        .expect("ThreadedFuture shared state shouldn't be poisoned");

                    if shared_state.kill {
                        return;
                    }

                    if shared_state.result.is_some() {
                        if let Some(waker) = shared_state.waker.take() {
                            waker.wake();
                        }
                    } else {
                        break;
                    }
                }
            }

            // Let the Stream implementation know that we're done
            let mut shared_state = thread_shared_state
                .lock()
                .expect("ThreadedFuture shared state shouldn't be poisoned");

            shared_state.result = Some(None);
            if let Some(waker) = shared_state.waker.take() {
                waker.wake();
            }
        });

        ThreadedReceiver { shared_state }
    }
}

impl<T: Send + 'static> Stream for ThreadedReceiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut shared_state = self
            .shared_state
            .lock()
            .expect("ThreadedFuture shared state shouldn't be poisoned");

        if let Some(result) = shared_state.result.take() {
            Poll::Ready(result)
        } else {
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T: Send + 'static> Drop for ThreadedReceiver<T> {
    fn drop(&mut self) {
        // Setting the kill flag to true will cause the thread spawned in `new` to exit, which
        // will cause the `Receiver` argument to get dropped. This can allow senders to
        // potentially clean up.
        match self.shared_state.lock() {
            Ok(mut state) => state.kill = true,
            Err(mut poisoned_err) => poisoned_err.get_mut().kill = true,
        }
    }
}

#[cfg(test)]
mod tests {
    mod threaded_receiver {
        use super::super::ThreadedReceiver;
        use futures::executor::block_on_stream;
        use std::sync::mpsc;

        #[test]
        fn returns_expected_result() {
            let (tx, rx) = mpsc::sync_channel(0);
            std::thread::spawn(move || {
                let _ = tx.send(1);
                let _ = tx.send(2);
                let _ = tx.send(3);
            });

            let stream = ThreadedReceiver::new(rx);
            let mut result = block_on_stream(stream);
            assert_eq!(Some(1), result.next());
            assert_eq!(Some(2), result.next());
            assert_eq!(Some(3), result.next());
            assert_eq!(None, result.next());
        }

        #[test]
        fn drops_receiver_when_stream_dropped() {
            let (tx, rx) = mpsc::sync_channel(0);
            let th = std::thread::spawn(move || {
                tx.send(1).and_then(|_| tx.send(2)).and_then(|_| tx.send(3))
            });

            {
                let stream = ThreadedReceiver::new(rx);
                let mut result = block_on_stream(stream);
                assert_eq!(Some(1), result.next());
            }
            let result = th.join();
            assert_eq!(true, result.unwrap().is_err());
        }
    }
}
