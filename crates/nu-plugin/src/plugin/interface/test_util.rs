use super::{EngineInterfaceManager, PluginInterfaceManager, PluginRead, PluginWrite};
use crate::{plugin::PluginSource, protocol::PluginInput, PluginOutput};
use nu_protocol::ShellError;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

/// Mock read/write helper for the engine and plugin interfaces.
#[derive(Debug, Clone)]
pub(crate) struct TestCase<I, O> {
    r#in: Arc<Mutex<TestData<I>>>,
    out: Arc<Mutex<TestData<O>>>,
}

#[derive(Debug)]
pub(crate) struct TestData<T> {
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
        let mut lock = self.r#in.lock().unwrap();
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
        let mut lock = self.out.lock().unwrap();
        lock.flushed = false;

        if let Some(err) = lock.error.take() {
            Err(err)
        } else {
            lock.data.push_back(data.clone());
            Ok(())
        }
    }

    fn flush(&self) -> Result<(), ShellError> {
        let mut lock = self.out.lock().unwrap();
        lock.flushed = true;
        Ok(())
    }
}

#[allow(dead_code)]
impl<I, O> TestCase<I, O> {
    pub(crate) fn new() -> TestCase<I, O> {
        TestCase {
            r#in: Default::default(),
            out: Default::default(),
        }
    }

    /// Clear the read buffer.
    pub(crate) fn clear(&self) {
        self.r#in.lock().unwrap().data.truncate(0);
    }

    /// Add input that will be read by the interface.
    pub(crate) fn add(&self, input: impl Into<I>) {
        self.r#in.lock().unwrap().data.push_back(input.into());
    }

    /// Add multiple inputs that will be read by the interface.
    pub(crate) fn extend(&self, inputs: impl IntoIterator<Item = I>) {
        self.r#in.lock().unwrap().data.extend(inputs);
    }

    /// Return an error from the next read operation.
    pub(crate) fn set_read_error(&self, err: ShellError) {
        self.r#in.lock().unwrap().error = Some(err);
    }

    /// Return an error from the next write operation.
    pub(crate) fn set_write_error(&self, err: ShellError) {
        self.out.lock().unwrap().error = Some(err);
    }

    /// Get the next output that was written.
    pub(crate) fn next_written(&self) -> Option<O> {
        self.out.lock().unwrap().data.pop_front()
    }

    /// Iterator over written data.
    pub(crate) fn written(&self) -> impl Iterator<Item = O> + '_ {
        std::iter::from_fn(|| self.next_written())
    }

    /// Returns true if the writer was flushed after the last write operation.
    pub(crate) fn was_flushed(&self) -> bool {
        self.out.lock().unwrap().flushed
    }

    /// Returns true if the reader has unconsumed reads.
    pub(crate) fn has_unconsumed_read(&self) -> bool {
        !self.r#in.lock().unwrap().data.is_empty()
    }

    /// Returns true if the writer has unconsumed writes.
    pub(crate) fn has_unconsumed_write(&self) -> bool {
        !self.out.lock().unwrap().data.is_empty()
    }
}

impl TestCase<PluginOutput, PluginInput> {
    /// Create a new [`PluginInterfaceManager`] that writes to this test case.
    pub(crate) fn plugin(&self, name: &str) -> PluginInterfaceManager {
        PluginInterfaceManager::new(PluginSource::new_fake(name).into(), None, self.clone())
    }
}

impl TestCase<PluginInput, PluginOutput> {
    /// Create a new [`EngineInterfaceManager`] that writes to this test case.
    pub(crate) fn engine(&self) -> EngineInterfaceManager {
        EngineInterfaceManager::new(self.clone())
    }
}
