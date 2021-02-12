use crate::plugin::run_plugin::PluginCommandBuilder;
use log::trace;
use nu_errors::ShellError;
use nu_plugin::jsonrpc::JsonRpc;
use nu_protocol::{Signature, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use rayon::prelude::*;

pub fn build_plugin_command(
    path: &std::path::Path,
) -> Result<Option<PluginCommandBuilder>, ShellError> {
    let ext = path.extension();
    let ps1_file = match ext {
        Some(ext) => ext == "ps1",
        None => false,
    };

    let mut child: Child = if ps1_file {
        Command::new("pwsh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .args(&[
                "-NoLogo",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &path.to_string_lossy(),
            ])
            .spawn()
            .expect("Failed to spawn PowerShell process")
    } else {
        Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process")
    };

    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    let stdout = child.stdout.as_mut().expect("Failed to open stdout");

    let mut reader = BufReader::new(stdout);

    let request = JsonRpc::new("config", Vec::<Value>::new());
    let request_raw = serde_json::to_string(&request)?;
    trace!(target: "nu::load", "plugin infrastructure config -> path {:#?}, request {:?}", &path, &request_raw);
    stdin.write_all(format!("{}\n", request_raw).as_bytes())?;
    let path = dunce::canonicalize(path)?;

    let mut input = String::new();
    let result = match reader.read_line(&mut input) {
        Ok(count) => {
            trace!(target: "nu::load", "plugin infrastructure -> config response for {:#?}", &path);
            trace!(target: "nu::load", "plugin infrastructure -> processing response ({} bytes)", count);
            trace!(target: "nu::load", "plugin infrastructure -> response: {}", input);

            let response = serde_json::from_str::<JsonRpc<Result<Signature, ShellError>>>(&input);
            match response {
                Ok(jrpc) => match jrpc.params {
                    Ok(params) => {
                        let fname = path.to_string_lossy();

                        trace!(target: "nu::load", "plugin infrastructure -> processing {:?}", params);

                        let name = params.name.clone();

                        let fname = fname.to_string();

                        Ok(Some(PluginCommandBuilder::new(&name, &fname, params)))
                    }
                    Err(e) => Err(e),
                },
                Err(e) => {
                    trace!(target: "nu::load", "plugin infrastructure -> incompatible {:?}", input);
                    Err(ShellError::untagged_runtime_error(format!(
                        "Error: {:?}",
                        e
                    )))
                }
            }
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!(
            "Error: {:?}",
            e
        ))),
    };

    let _ = child.wait();

    result
}

pub fn scan(
    paths: Vec<std::path::PathBuf>,
) -> Result<Vec<crate::whole_stream_command::Command>, ShellError> {
    let mut plugins = vec![];

    let opts = glob::MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    for path in paths {
        let mut pattern = path.to_path_buf();

        pattern.push(std::path::Path::new("nu_plugin_[a-z0-9][a-z0-9]*"));

        let plugs: Vec<_> = glob::glob_with(&pattern.to_string_lossy(), opts)?
            .filter_map(|x| x.ok())
            .collect();

        let plugs: Vec<_> = plugs
            .par_iter()
            .filter_map(|path| {
                let bin_name = {
                    if let Some(name) = path.file_name() {
                        name.to_str().unwrap_or("")
                    } else {
                        ""
                    }
                };

                // allow plugins with extensions on all platforms
                let is_valid_name = {
                    bin_name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                };

                let is_executable = {
                    #[cfg(windows)]
                    {
                        bin_name.ends_with(".exe") 
                        || bin_name.ends_with(".bat")
                        || bin_name.ends_with(".cmd")
                        || bin_name.ends_with(".py")
                        || bin_name.ends_with(".ps1")
                    }

                    #[cfg(not(windows))]
                    {
                        !bin_name.contains('.')
                        || (bin_name.ends_with('.')
                        || bin_name.ends_with(".py")
                        || bin_name.ends_with(".rb")
                        || bin_name.ends_with(".sh")
                        || bin_name.ends_with(".bash")
                        || bin_name.ends_with(".zsh")
                        || bin_name.ends_with(".pl")
                        || bin_name.ends_with(".awk")
                        || bin_name.ends_with(".ps1"))
                    }
                };

                if is_valid_name && is_executable {
                    trace!(target: "nu::load", "plugin infrastructure -> Trying {:?}", path.display());
                    build_plugin_command(&path).unwrap_or(None)
                } else {
                    None
                }
            }).map(|p| p.build())
            .collect::<Vec<crate::whole_stream_command::Command>>();
        plugins.extend(plugs);
    }

    Ok(plugins)
}
