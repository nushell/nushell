use std::ffi::OsStr;
use std::io::{Stdin, Stdout};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use nu_protocol::ShellError;

#[cfg(feature = "local-socket")]
use interprocess::local_socket::LocalSocketListener;

#[cfg(feature = "local-socket")]
mod local_socket;

#[cfg(feature = "local-socket")]
use local_socket::*;

#[derive(Debug, Clone)]
pub(crate) enum CommunicationMode {
    /// Communicate using `stdin` and `stdout`.
    Stdio,
    /// Communicate using an operating system-specific local socket.
    #[cfg(feature = "local-socket")]
    LocalSocket(std::ffi::OsString),
}

impl CommunicationMode {
    /// Generate a new local socket communication mode based on the given plugin exe path.
    #[cfg(feature = "local-socket")]
    pub fn local_socket(plugin_exe: &std::path::Path) -> CommunicationMode {
        use std::hash::{Hash, Hasher};
        use std::time::SystemTime;

        // Generate the unique ID based on the plugin path and the current time. The actual
        // algorithm here is not very important, we just want this to be relatively unique very
        // briefly. Using the default hasher in the stdlib means zero extra dependencies.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        plugin_exe.hash(&mut hasher);
        SystemTime::now().hash(&mut hasher);

        let unique_id = format!("{:016x}", hasher.finish());

        CommunicationMode::LocalSocket(make_local_socket_name(&unique_id))
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
            CommunicationMode::LocalSocket(name) => {
                let listener = LocalSocketListener::bind(name.as_os_str()).map_err(|err| {
                    ShellError::IOError {
                        msg: format!("failed to open socket for plugin: {err}"),
                    }
                })?;
                Ok(PreparedServerCommunication::LocalSocket {
                    name: name.clone(),
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
            CommunicationMode::LocalSocket(name) => {
                // Connect to the specified socket.
                let get_socket = || {
                    use interprocess::local_socket as ls;
                    ls::LocalSocketStream::connect(name.as_os_str())
                        .map_err(|err| ShellError::IOError {
                            msg: format!("failed to connect to socket: {err}"),
                        })
                        .map(LocalSocketStream::from)
                };
                // Reverse order from the server: read in, write out
                let read_in = get_socket()?;
                let write_out = get_socket()?;
                Ok(ClientCommunicationIo::LocalSocket { read_in, write_out })
            }
        }
    }
}

pub(crate) enum PreparedServerCommunication {
    Stdio,
    #[cfg(feature = "local-socket")]
    LocalSocket {
        #[cfg_attr(windows, allow(dead_code))] // not used on Windows
        name: std::ffi::OsString,
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

                const RETRY_PERIOD: Duration = Duration::from_millis(1);
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
                                if !is_would_block_err(&err) {
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
                        Ok(LocalSocketStream(stream))
                    } else {
                        // The process may have exited
                        Err(ShellError::PluginFailedToLoad {
                            msg: "Plugin exited without connecting".into(),
                        })
                    }
                };
                // Input stream always comes before output
                let write_in = get_socket()?;
                let read_out = get_socket()?;
                Ok(ServerCommunicationIo::LocalSocket { read_out, write_in })
            }
        }
    }
}

impl Drop for PreparedServerCommunication {
    fn drop(&mut self) {
        match self {
            #[cfg(all(unix, feature = "local-socket"))]
            PreparedServerCommunication::LocalSocket { name: path, .. } => {
                // Just try to remove the socket file, it's ok if this fails
                let _ = std::fs::remove_file(path);
            }
            _ => (),
        }
    }
}

pub(crate) enum ServerCommunicationIo {
    Stdio(ChildStdin, ChildStdout),
    #[cfg(feature = "local-socket")]
    LocalSocket {
        read_out: LocalSocketStream,
        write_in: LocalSocketStream,
    },
}

pub(crate) enum ClientCommunicationIo {
    Stdio(Stdin, Stdout),
    #[cfg(feature = "local-socket")]
    LocalSocket {
        read_in: LocalSocketStream,
        write_out: LocalSocketStream,
    },
}
