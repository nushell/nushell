use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{self, Poll, Waker};
use std::thread;

struct SharedState<T: Send + 'static> {
    result: Option<T>,
    waker: Option<Waker>,
}

pub struct ThreadedFuture<T: Send + 'static> {
    shared_state: Arc<Mutex<SharedState<T>>>,
}

impl<T: Send + 'static> ThreadedFuture<T> {
    pub fn new(f: impl FnOnce() -> T + Send + 'static) -> ThreadedFuture<T> {
        let shared_state = Arc::new(Mutex::new(SharedState {
            result: None,
            waker: None,
        }));

        // Clone everything to avoid lifetimes
        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            let result = f();

            let mut shared_state = thread_shared_state
                .lock()
                .expect("ThreadedFuture shared state shouldn't be poisoned");

            shared_state.result = Some(result);
            if let Some(waker) = shared_state.waker.take() {
                waker.wake();
            }
        });

        ThreadedFuture { shared_state }
    }
}

impl<T: Send + 'static> Future for ThreadedFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
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

#[cfg(test)]
mod tests {
    mod threaded_future {
        use super::super::ThreadedFuture;
        use futures::executor::block_on;

        #[test]
        fn returns_expected_result() {
            let future = ThreadedFuture::new(|| 42);
            let result = block_on(future);
            assert_eq!(42, result);
        }
    }
}
