use nu_engine::command_prelude::*;
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

pub fn edit_value_with_editor(
    value: &Value,
    engine_state: &EngineState,
    _stack: &mut Stack,
    span: Span,
) -> Result<Value, ShellError> {
    // Create a temporary file with the value content
    let mut temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| ShellError::GenericError { 
            error: "Failed to create temp file".to_string(), 
            msg: format!("{}", e), 
            span: Some(span), 
            help: None, 
            inner: vec![] 
        })?;
    
    // Convert value to string representation
    let content = match value {
        Value::String { val, .. } => val.clone(),
        _ => {
            // For non-string values, use the display representation
            format!("{}", value.to_abbreviated_string(engine_state.get_config()))
        }
    };
    
    temp_file
        .write_all(content.as_bytes())
        .map_err(|e| ShellError::GenericError { 
            error: "Failed to write to temp file".to_string(), 
            msg: format!("{}", e), 
            span: Some(span), 
            help: None, 
            inner: vec![] 
        })?;
    
    let temp_path = temp_file.path();
    
    // Get editor from environment or use default
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    
    // Open the editor
    let mut child = Command::new(&editor)
        .arg(temp_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| ShellError::GenericError { 
            error: "Failed to start editor".to_string(), 
            msg: format!("Editor: {}, Error: {}", editor, e), 
            span: Some(span), 
            help: None, 
            inner: vec![] 
        })?;
    
    // Wait for editor to finish
    let status = child
        .wait()
        .map_err(|e| ShellError::GenericError { 
            error: "Failed to wait for editor".to_string(), 
            msg: format!("{}", e), 
            span: Some(span), 
            help: None, 
            inner: vec![] 
        })?;
    
    if !status.success() {
        return Err(ShellError::GenericError { 
            error: "Editor exited with non-zero status".to_string(), 
            msg: "Editor did not complete successfully".to_string(), 
            span: Some(span), 
            help: None, 
            inner: vec![] 
        });
    }
    
    // Read the modified content
    let modified_content = fs::read_to_string(temp_path)
        .map_err(|e| ShellError::GenericError { 
            error: "Failed to read modified file".to_string(), 
            msg: format!("{}", e), 
            span: Some(span), 
            help: None, 
            inner: vec![] 
        })?;
    
    // Return the modified content as a string value
    Ok(Value::string(modified_content, span))
}