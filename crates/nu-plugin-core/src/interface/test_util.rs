use nu_protocol::ShellError;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{PluginRead, PluginWrite};

const FAILED: &str = "failed to lock TestCase";

/// Mock read/write helper for the engine and plugin interfaces.
#[derive(Debug, Clone)]
pub struct TestCase<I, O> {
    r#in: Arc<Mutex<TestData<I>>>,
    out: Arc<Mutex<TestData<O>>>,
}

#[derive(Debug)]
pub struct TestData<T> {
    data: VecDeque<T>,
    error: Option<ShellError>,
    flushed: bool,
}

impl<T> Default for TestData<T> {
    fn default() -> Self {
        TestData {
            data: VecDeque::new(),
            error: None,
            flushed: false,
        }
    }
}

impl<I, O> PluginRead<I> for TestCase<I, O> {
    fn read(&mut self) -> Result<Option<I>, ShellError> {
        let mut lock = self.r#in.lock().expect(FAILED);
        if let Some(err) = lock.error.take() {
            Err(err)
        } else {
            Ok(lock.data.pop_front())
        }
    }
}

impl<I, O> PluginWrite<O> for TestCase<I, O>
where
    I: Send + Clone,
    O: Send + Clone,
{
    fn write(&self, data: &O) -> Result<(), ShellError> {
        let mut lock = self.out.lock().expect(FAILED);
        lock.flushed = false;

        if let Some(err) = lock.error.take() {
            Err(err)
        } else {
            lock.data.push_back(data.clone());
            Ok(())
        }
    }

    fn flush(&self) -> Result<(), ShellError> {
        let mut lock = self.out.lock().expect(FAILED);
        lock.flushed = true;
        Ok(())
    }
}

#[allow(dead_code)]
impl<I, O> TestCase<I, O> {
    pub fn new() -> TestCase<I, O> {
        TestCase {
            r#in: Default::default(),
            out: Default::default(),
        }
    }

    /// Clear the read buffer.
    pub fn clear(&self) {
        self.r#in.lock().expect(FAILED).data.truncate(0);
    }

    /// Add input that will be read by the interface.
    pub fn add(&self, input: impl Into<I>) {
        self.r#in.lock().expect(FAILED).data.push_back(input.into());
    }

    /// Add multiple inputs that will be read by the interface.
    pub fn extend(&self, inputs: impl IntoIterator<Item = I>) {
        self.r#in.lock().expect(FAILED).data.extend(inputs);
    }

    /// Return an error from the next read operation.
    pub fn set_read_error(&self, err: ShellError) {
        self.r#in.lock().expect(FAILED).error = Some(err);
    }

    /// Return an error from the next write operation.
    pub fn set_write_error(&self, err: ShellError) {
        self.out.lock().expect(FAILED).error = Some(err);
    }

    /// Get the next output that was written.
    pub fn next_written(&self) -> Option<O> {
        self.out.lock().expect(FAILED).data.pop_front()
    }

    /// Iterator over written data.
    pub fn written(&self) -> impl Iterator<Item = O> + '_ {
        std::iter::from_fn(|| self.next_written())
    }

    /// Returns true if the writer was flushed after the last write operation.
    pub fn was_flushed(&self) -> bool {
        self.out.lock().expect(FAILED).flushed
    }

    /// Returns true if the reader has unconsumed reads.
    pub fn has_unconsumed_read(&self) -> bool {
        !self.r#in.lock().expect(FAILED).data.is_empty()
    }

    /// Returns true if the writer has unconsumed writes.
    pub fn has_unconsumed_write(&self) -> bool {
        !self.out.lock().expect(FAILED).data.is_empty()
    }
}

impl<I, O> Default for TestCase<I, O> {
    fn default() -> Self {
        Self::new()
    }
}
