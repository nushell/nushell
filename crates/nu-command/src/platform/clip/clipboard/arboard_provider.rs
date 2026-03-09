use super::error_mapper::map_arboard_err_to_shell;
use nu_protocol::ShellError;

pub(crate) fn with_clipboard_instance<
    U,
    F: FnOnce(&mut arboard::Clipboard) -> Result<U, arboard::Error>,
>(
    op: F,
) -> Result<U, ShellError> {
    let mut clipboard = arboard::Clipboard::new().map_err(map_arboard_err_to_shell)?;

    op(&mut clipboard).map_err(map_arboard_err_to_shell)
}
