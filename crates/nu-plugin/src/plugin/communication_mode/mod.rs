use std::ffi::OsStr;
use std::io::{Stdin, Stdout};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use nu_protocol::ShellError;

#[cfg(feature = "local-socket")]
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};

#[cfg(feature = "local-socket")]
mod local_socket;

#[derive(Debug, Clone)]
pub(crate) enum CommunicationMode {
    /// Communicate using `stdin` and `stdout`.
    Stdio,
    /// Communicate using an operating system-specific local socket.
    #[cfg(feature = "local-socket")]
    LocalSocket(std::path::PathBuf),
}

impl CommunicationMode {
    /// Generate a new local socket communication mode based on the given plugin exe path.
    #[cfg(feature = "local-socket")]
    pub fn local_socket(plugin_exe: &std::path::Path) -> CommunicationMode {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        let plugin_exe_name = plugin_exe
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|| std::borrow::Cow::Borrowed("unknown_plugin"));

        let unique_id = format!("{}.{}", plugin_exe_name, timestamp);

        CommunicationMode::LocalSocket(local_socket::make_local_socket_path(&unique_id))
    }

    pub fn args(&self) -> Vec<&OsStr> {
        match self {
            CommunicationMode::Stdio => vec![OsStr::new("--stdio")],
            #[cfg(feature = "local-socket")]
            CommunicationMode::LocalSocket(path) => {
                vec![OsStr::new("--local-socket"), path.as_os_str()]
            }
        }
    }

    pub fn setup_command_io(&self, command: &mut Command) {
        match self {
            CommunicationMode::Stdio => {
                // Both stdout and stdin are piped so we can receive information from the plugin
                command.stdin(Stdio::piped());
                command.stdout(Stdio::piped());
            }
            #[cfg(feature = "local-socket")]
            CommunicationMode::LocalSocket(_) => {
                // Stdio can be used by the plugin to talk to the terminal in local socket mode,
                // which is the big benefit
                command.stdin(Stdio::inherit());
                command.stdout(Stdio::inherit());
            }
        }
    }

    pub fn serve(&self) -> Result<PreparedServerCommunication, ShellError> {
        match self {
            // Nothing to set up for stdio - we just take it from the child.
            CommunicationMode::Stdio => Ok(PreparedServerCommunication::Stdio),
            // For sockets: we need to create the server so that the child won't fail to connect.
            #[cfg(feature = "local-socket")]
            CommunicationMode::LocalSocket(path) => {
                let listener = LocalSocketListener::bind(path.as_path()).map_err(|err| {
                    ShellError::IOError {
                        msg: format!("failed to open socket for plugin: {err}"),
                    }
                })?;
                Ok(PreparedServerCommunication::LocalSocket {
                    path: path.clone(),
                    listener,
                })
            }
        }
    }

    pub fn connect_as_client(&self) -> Result<ClientCommunicationIo, ShellError> {
        match self {
            CommunicationMode::Stdio => Ok(ClientCommunicationIo::Stdio(
                std::io::stdin(),
                std::io::stdout(),
            )),
            #[cfg(feature = "local-socket")]
            CommunicationMode::LocalSocket(path) => {
                // Connect to the specified socket.
                let get_socket = || {
                    LocalSocketStream::connect(path.as_path()).map_err(|err| ShellError::IOError {
                        msg: format!("failed to connect to socket: {err}"),
                    })
                };
                // Reverse order from the server: write first (plugin output), then read (plugin
                // input)
                let write = get_socket()?;
                let read = get_socket()?;
                Ok(ClientCommunicationIo::LocalSocket { read, write })
            }
        }
    }
}

pub(crate) enum PreparedServerCommunication {
    Stdio,
    #[cfg(feature = "local-socket")]
    LocalSocket {
        path: std::path::PathBuf,
        listener: LocalSocketListener,
    },
}

impl PreparedServerCommunication {
    pub fn connect(&self, child: &mut Child) -> Result<ServerCommunicationIo, ShellError> {
        match self {
            PreparedServerCommunication::Stdio => {
                let stdin = child
                    .stdin
                    .take()
                    .ok_or_else(|| ShellError::PluginFailedToLoad {
                        msg: "Plugin missing stdin writer".into(),
                    })?;

                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| ShellError::PluginFailedToLoad {
                        msg: "Plugin missing stdout writer".into(),
                    })?;

                Ok(ServerCommunicationIo::Stdio(stdin, stdout))
            }
            #[cfg(feature = "local-socket")]
            PreparedServerCommunication::LocalSocket { listener, .. } => {
                use std::time::{Duration, Instant};

                const RETRY_PERIOD: Duration = Duration::from_millis(10);
                const TIMEOUT: Duration = Duration::from_secs(10);

                let start = Instant::now();

                // Use a loop to try to get two clients from the listener: one for read (the plugin
                // output) and one for write (the plugin input)
                listener.set_nonblocking(true)?;
                let mut get_socket = || {
                    let mut result = None;
                    while let Ok(None) = child.try_wait() {
                        match listener.accept() {
                            Ok(stream) => {
                                // Success! But make sure the stream is in blocking mode.
                                stream.set_nonblocking(false)?;
                                result = Some(stream);
                                break;
                            }
                            Err(err) => {
                                if err.kind() != std::io::ErrorKind::WouldBlock {
                                    // `WouldBlock` is ok, just means it's not ready yet, but some other
                                    // kind of error should be reported
                                    return Err(err.into());
                                }
                            }
                        }
                        if Instant::now().saturating_duration_since(start) > TIMEOUT {
                            return Err(ShellError::PluginFailedToLoad {
                                msg: "Plugin timed out while waiting to connect to socket".into(),
                            });
                        } else {
                            std::thread::sleep(RETRY_PERIOD);
                        }
                    }
                    if let Some(stream) = result {
                        Ok(stream)
                    } else {
                        // The process may have exited
                        Err(ShellError::PluginFailedToLoad {
                            msg: "Plugin exited without connecting".into(),
                        })
                    }
                };
                let read = get_socket()?;
                let write = get_socket()?;
                Ok(ServerCommunicationIo::LocalSocket { read, write })
            }
        }
    }
}

impl Drop for PreparedServerCommunication {
    fn drop(&mut self) {
        match self {
            PreparedServerCommunication::Stdio => (),
            PreparedServerCommunication::LocalSocket { path, .. } => {
                // Just try to remove the socket file, it's ok if this fails
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

pub(crate) enum ServerCommunicationIo {
    Stdio(ChildStdin, ChildStdout),
    #[cfg(feature = "local-socket")]
    LocalSocket {
        read: LocalSocketStream,
        write: LocalSocketStream,
    },
}

pub(crate) enum ClientCommunicationIo {
    Stdio(Stdin, Stdout),
    #[cfg(feature = "local-socket")]
    LocalSocket {
        read: LocalSocketStream,
        write: LocalSocketStream,
    },
}
