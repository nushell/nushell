use nu_protocol::{
    Config, ShellError,
    engine::{EngineState, Stack},
};

use super::arboard_provider::with_clipboard_instance;

#[cfg(target_os = "linux")]
pub fn create_clipboard(
    config: &Config,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> impl Clipboard {
    super::linux::ClipBoardLinux::new(config, engine_state, stack)
}

#[cfg(not(target_os = "linux"))]
pub fn create_clipboard(
    _config: &Config,
    _engine_state: &EngineState,
    _stack: &mut Stack,
) -> impl Clipboard {
    #[cfg(target_os = "macos")]
    {
        super::mac_os::ClipBoardMacos::new()
    }
    #[cfg(target_os = "windows")]
    {
        super::windows::ClipBoardWindows::new()
    }
}

pub trait Clipboard {
    fn copy_text(&self, text: &str) -> Result<(), ShellError> {
        with_clipboard_instance(|clip| clip.set_text(text))
    }

    fn get_text(&self) -> Result<String, ShellError> {
        with_clipboard_instance(|clip| clip.get_text())
    }
}
