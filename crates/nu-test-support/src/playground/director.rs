use super::nu_process::*;
use std::ffi::OsString;
use std::fmt;

#[derive(Default, Debug)]
pub struct Director {
    pub cwd: Option<OsString>,
    pub config: Option<OsString>,
    pub pipeline: Option<String>,
    pub executable: Option<NuProcess>,
}

impl Director {
    pub fn cococo(&self, arg: &str) -> Self {
        let mut process = NuProcess::default();
        process.args(&["--testbin", "cococo", arg]);
        Director {
            config: self.config.clone(),
            executable: Some(process),
            ..Default::default()
        }
    }

    pub fn pipeline(&self, commands: &str) -> Self {
        let mut director = Director {
            pipeline: if commands.is_empty() {
                None
            } else {
                Some(format!(
                    "
                                {}
                                exit",
                    commands
                ))
            },
            ..Default::default()
        };

        let mut process = NuProcess::default();

        if let Some(working_directory) = &self.cwd {
            process.cwd(working_directory);
        }

        process.arg("--skip-plugins");
        if let Some(config_file) = self.config.as_ref() {
            process.args(&[
                "--config-file",
                config_file.to_str().expect("failed to convert."),
            ]);
        }

        director.executable = Some(process);
        director
    }

    pub fn executable(&self) -> Option<&NuProcess> {
        if let Some(binary) = &self.executable {
            Some(binary)
        } else {
            None
        }
    }
}

impl Executable for Director {
    fn execute(&self) -> NuResult {
        use std::io::Write;
        use std::process::Stdio;

        match self.executable() {
            Some(binary) => {
                let mut process = match binary
                    .construct()
                    .stdout(Stdio::piped())
                    .stdin(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                {
                    Ok(child) => child,
                    Err(why) => panic!("Can't run test {}", why.to_string()),
                };

                if let Some(pipeline) = &self.pipeline {
                    process
                        .stdin
                        .as_mut()
                        .expect("couldn't open stdin")
                        .write_all(pipeline.as_bytes())
                        .expect("couldn't write to stdin");
                }

                process
                    .wait_with_output()
                    .map_err(|_| {
                        let reason = format!(
                            "could not execute process {} ({})",
                            binary, "No execution took place"
                        );

                        NuError {
                            desc: reason,
                            exit: None,
                            output: None,
                        }
                    })
                    .and_then(|process| {
                        let out =
                            Outcome::new(&read_std(&process.stdout), &read_std(&process.stderr));

                        match process.status.success() {
                            true => Ok(out),
                            false => Err(NuError {
                                desc: String::new(),
                                exit: Some(process.status),
                                output: Some(out),
                            }),
                        }
                    })
            }
            None => Err(NuError {
                desc: String::from("err"),
                exit: None,
                output: None,
            }),
        }
    }
}

impl fmt::Display for Director {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "director")
    }
}

fn read_std(std: &[u8]) -> Vec<u8> {
    let out = String::from_utf8_lossy(std);
    let out = out.lines().collect::<Vec<_>>().join("\n");
    let out = out.replace("\r\n", "");
    out.replace("\n", "").into_bytes()
}
