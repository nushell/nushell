use super::nu_process::*;
use super::EnvironmentVariable;
use std::ffi::OsString;
use std::fmt;
use std::fmt::Write;

#[derive(Default, Debug)]
pub struct Director {
    pub cwd: Option<OsString>,
    pub environment_vars: Vec<EnvironmentVariable>,
    pub config: Option<OsString>,
    pub pipeline: Option<Vec<String>>,
    pub executable: Option<NuProcess>,
}

impl Director {
    pub fn cococo(&self, arg: &str) -> Self {
        let mut process = NuProcess {
            environment_vars: self.environment_vars.clone(),
            ..Default::default()
        };

        process.args(&["--testbin", "cococo", arg]);
        Director {
            config: self.config.clone(),
            executable: Some(process),
            environment_vars: self.environment_vars.clone(),
            ..Default::default()
        }
    }

    pub fn and_then(&mut self, commands: &str) -> &mut Self {
        let commands = commands.to_string();

        if let Some(ref mut pipeline) = self.pipeline {
            pipeline.push(commands);
        } else {
            self.pipeline = Some(vec![commands]);
        }

        self
    }

    pub fn pipeline(&self, commands: &str) -> Self {
        let mut director = Director {
            pipeline: if commands.is_empty() {
                None
            } else {
                Some(vec![commands.to_string()])
            },
            ..Default::default()
        };

        let mut process = NuProcess {
            environment_vars: self.environment_vars.clone(),
            ..Default::default()
        };

        if let Some(working_directory) = &self.cwd {
            process.cwd(working_directory);
        }

        process.arg("--skip-plugins");
        process.arg("--no-history");
        if let Some(config_file) = self.config.as_ref() {
            process.args(&[
                "--config-file",
                config_file.to_str().expect("failed to convert."),
            ]);
        }
        process.arg("--perf");

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
    fn execute(&mut self) -> NuResult {
        use std::process::Stdio;

        match self.executable() {
            Some(binary) => {
                let mut commands = String::new();
                if let Some(pipelines) = &self.pipeline {
                    for pipeline in pipelines {
                        if !commands.is_empty() {
                            commands.push_str("| ");
                        }
                        let _ = writeln!(commands, "{}", pipeline);
                    }
                }

                let process = match binary
                    .construct()
                    .stdout(Stdio::piped())
                    // .stdin(Stdio::piped())
                    .stderr(Stdio::piped())
                    .arg(format!("-c '{}'", commands))
                    .spawn()
                {
                    Ok(child) => child,
                    Err(why) => panic!("Can't run test {}", why),
                };

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
    out.replace('\n', "").into_bytes()
}
