use std::sync::{mpsc, Arc};

use nu_plugin::Plugin;
use nu_plugin_core::{InterfaceManager, PluginRead, PluginWrite};
use nu_plugin_engine::{PluginInterfaceManager, PluginSource};
use nu_plugin_protocol::{PluginInput, PluginOutput};
use nu_protocol::{shell_error::io::IoError, PluginIdentity, ShellError};

use crate::fake_persistent_plugin::FakePersistentPlugin;

struct FakePluginRead<T>(mpsc::Receiver<T>);
struct FakePluginWrite<T>(mpsc::Sender<T>);

impl<T> PluginRead<T> for FakePluginRead<T> {
    fn read(&mut self) -> Result<Option<T>, ShellError> {
        Ok(self.0.recv().ok())
    }
}

impl<T: Clone + Send> PluginWrite<T> for FakePluginWrite<T> {
    fn write(&self, data: &T) -> Result<(), ShellError> {
        self.0
            .send(data.clone())
            .map_err(|e| ShellError::GenericError {
                error: "Error sending data".to_string(),
                msg: e.to_string(),
                span: None,
                help: None,
                inner: vec![],
            })
    }

    fn flush(&self) -> Result<(), ShellError> {
        Ok(())
    }
}

fn fake_plugin_channel<T: Clone + Send>() -> (FakePluginRead<T>, FakePluginWrite<T>) {
    let (tx, rx) = mpsc::channel();
    (FakePluginRead(rx), FakePluginWrite(tx))
}

/// Spawn a plugin on another thread and return the registration
pub(crate) fn spawn_fake_plugin(
    name: &str,
    plugin: Arc<impl Plugin + Send + 'static>,
) -> Result<Arc<FakePersistentPlugin>, ShellError> {
    let (input_read, input_write) = fake_plugin_channel::<PluginInput>();
    let (output_read, output_write) = fake_plugin_channel::<PluginOutput>();

    let identity = PluginIdentity::new_fake(name);
    let reg_plugin = Arc::new(FakePersistentPlugin::new(identity.clone()));
    let source = Arc::new(PluginSource::new(reg_plugin.clone()));

    // The fake plugin has no process ID, and we also don't set the garbage collector
    let mut manager = PluginInterfaceManager::new(source, None, input_write);

    // Set up the persistent plugin with the interface before continuing
    let interface = manager.get_interface();
    interface.hello()?;
    reg_plugin.initialize(interface);

    // Start the interface reader on another thread
    std::thread::Builder::new()
        .name(format!("fake plugin interface reader ({name})"))
        .spawn(move || manager.consume_all(output_read).expect("Plugin read error"))
        .map_err(|err| {
            IoError::new_internal(
                err.kind(),
                format!("Could not spawn fake plugin interface reader ({name})"),
                nu_protocol::location!(),
            )
        })?;

    // Start the plugin on another thread
    let name_string = name.to_owned();
    std::thread::Builder::new()
        .name(format!("fake plugin runner ({name})"))
        .spawn(move || {
            nu_plugin::serve_plugin_io(
                &*plugin,
                &name_string,
                move || input_read,
                move || output_write,
            )
            .expect("Plugin runner error")
        })
        .map_err(|err| {
            IoError::new_internal(
                err.kind(),
                format!("Could not spawn fake plugin runner ({name})"),
                nu_protocol::location!(),
            )
        })?;

    Ok(reg_plugin)
}
