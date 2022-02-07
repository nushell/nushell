pub fn keybinding_path() -> Result<std::path::PathBuf, nu_errors::ShellError> {
    crate::config::default_path_for(&Some(std::path::PathBuf::from("keybindings.yml")))
}
