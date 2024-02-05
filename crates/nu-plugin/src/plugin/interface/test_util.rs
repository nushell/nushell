use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use nu_protocol::ShellError;

use crate::protocol::{PluginInput, PluginOutput};

use super::{
    engine::EngineInterfaceImpl, plugin::PluginInterfaceImpl, EngineInterface,
    PluginExecutionContext, PluginInterface, PluginRead, PluginWrite,
};

/// Mock read/write helper for the engine and plugin interfaces.
#[derive(Debug, Clone)]
pub(crate) struct TestCase {
    r#in: Arc<Mutex<TestData>>,
    out: Arc<Mutex<TestData>>,
}

#[derive(Debug, Default)]
pub(crate) struct TestData {
    inputs: VecDeque<PluginInput>,
    outputs: VecDeque<PluginOutput>,
    error: Option<ShellError>,
    flushed: bool,
}

type TestIo = Arc<Mutex<TestData>>;

impl PluginRead for TestIo {
    fn read_input(&mut self) -> Result<Option<PluginInput>, ShellError> {
        let mut lock = self.lock().unwrap();
        if let Some(err) = lock.error.take() {
            Err(err)
        } else {
            Ok(lock.inputs.pop_front())
        }
    }

    fn read_output(&mut self) -> Result<Option<PluginOutput>, ShellError> {
        let mut lock = self.lock().unwrap();
        if let Some(err) = lock.error.take() {
            Err(err)
        } else {
            Ok(lock.outputs.pop_front())
        }
    }
}

impl PluginWrite for TestIo {
    fn write_input(&mut self, input: &PluginInput) -> Result<(), ShellError> {
        let mut lock = self.lock().unwrap();
        lock.flushed = false;

        if let Some(err) = lock.error.take() {
            Err(err)
        } else {
            lock.inputs.push_back(input.clone());
            Ok(())
        }
    }

    fn write_output(&mut self, output: &PluginOutput) -> Result<(), ShellError> {
        let mut lock = self.lock().unwrap();
        lock.flushed = false;

        if let Some(err) = lock.error.take() {
            Err(err)
        } else {
            lock.outputs.push_back(output.clone());
            Ok(())
        }
    }

    fn flush(&mut self) -> Result<(), ShellError> {
        let mut lock = self.lock().unwrap();
        lock.flushed = true;
        Ok(())
    }
}

#[allow(dead_code)]
impl TestCase {
    pub(crate) fn new() -> TestCase {
        TestCase {
            r#in: Default::default(),
            out: Default::default(),
        }
    }

    /// Clear the input read buffer.
    pub(crate) fn clear_input(&self) {
        self.r#in.lock().unwrap().inputs.truncate(0);
    }

    /// Clear the output read buffer.
    pub(crate) fn clear_output(&self) {
        self.r#in.lock().unwrap().outputs.truncate(0);
    }

    /// Add input that will be read by the interface.
    pub(crate) fn add_input(&self, input: PluginInput) {
        self.r#in.lock().unwrap().inputs.push_back(input);
    }

    /// Add output that will be read by the interface.
    pub(crate) fn add_output(&self, output: PluginOutput) {
        self.r#in.lock().unwrap().outputs.push_back(output);
    }

    /// Add multiple inputs that will be read by the interface.
    pub(crate) fn extend_input(&self, inputs: impl IntoIterator<Item = PluginInput>) {
        self.r#in.lock().unwrap().inputs.extend(inputs);
    }

    /// Add multiple outputs that will be read by the interface.
    pub(crate) fn extend_output(&self, outputs: impl IntoIterator<Item = PluginOutput>) {
        self.r#in.lock().unwrap().outputs.extend(outputs);
    }

    /// Return an error from the next read operation.
    pub(crate) fn set_read_error(&self, err: ShellError) {
        self.r#in.lock().unwrap().error = Some(err);
    }

    /// Return an error from the next write operation.
    pub(crate) fn set_write_error(&self, err: ShellError) {
        self.out.lock().unwrap().error = Some(err);
    }

    /// Get the next input that was written.
    pub(crate) fn next_written_input(&self) -> Option<PluginInput> {
        self.out.lock().unwrap().inputs.pop_front()
    }

    /// Get the next output that was written.
    pub(crate) fn next_written_output(&self) -> Option<PluginOutput> {
        self.out.lock().unwrap().outputs.pop_front()
    }

    /// Iterator over written inputs.
    pub(crate) fn written_inputs(&self) -> impl Iterator<Item = PluginInput> + '_ {
        std::iter::from_fn(|| self.next_written_input())
    }

    /// Iterator over written outputs.
    pub(crate) fn written_outputs(&self) -> impl Iterator<Item = PluginOutput> + '_ {
        std::iter::from_fn(|| self.next_written_output())
    }

    /// Returns true if the writer was flushed after the last write operation.
    pub(crate) fn was_flushed(&self) -> bool {
        self.out.lock().unwrap().flushed
    }

    /// Returns true if the reader has unconsumed read input/output.
    pub(crate) fn has_unconsumed_read(&self) -> bool {
        let lock = self.r#in.lock().unwrap();
        !lock.inputs.is_empty() || !lock.outputs.is_empty()
    }

    /// Returns true if the writer has unconsumed write input/output.
    pub(crate) fn has_unconsumed_write(&self) -> bool {
        let lock = self.out.lock().unwrap();
        !lock.inputs.is_empty() || !lock.outputs.is_empty()
    }

    /// Create an [EngineInterfaceImpl] using the test data.
    pub(crate) fn engine_interface_impl(&self) -> EngineInterfaceImpl<TestIo, TestIo> {
        EngineInterfaceImpl::new(self.r#in.clone(), self.out.clone())
    }

    /// Create an [EngineInterface] using the test data.
    pub(crate) fn engine_interface(&self) -> EngineInterface {
        self.engine_interface_impl().into()
    }

    /// Create a [PluginInterfaceImpl] using the test data.
    pub(crate) fn plugin_interface_impl(
        &self,
        context: Option<Arc<dyn PluginExecutionContext>>,
    ) -> PluginInterfaceImpl<TestIo, TestIo> {
        PluginInterfaceImpl::new(self.r#in.clone(), self.out.clone(), context)
    }

    /// Create a [PluginInterface] using the test data.
    pub(crate) fn plugin_interface(
        &self,
        context: Option<Arc<dyn PluginExecutionContext>>,
    ) -> PluginInterface {
        self.plugin_interface_impl(context).into()
    }
}
