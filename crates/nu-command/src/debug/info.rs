use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Span,
    Type, Value,
};
use sysinfo::{Pid, PidExt, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};
// Character used to separate directories in a Path Environment variable on windows is ";"
#[cfg(target_family = "windows")]
const ENV_PATH_SEPARATOR_CHAR: char = ';';
// Character used to separate directories in a Path Environment variable on linux/mac/unix is ":"
#[cfg(not(target_family = "windows"))]
const ENV_PATH_SEPARATOR_CHAR: char = ':';

#[derive(Clone)]
pub struct DebugInfo;

impl Command for DebugInfo {
    fn name(&self) -> &str {
        "debug info"
    }

    fn usage(&self) -> &str {
        "View process memory info."
    }

    fn extra_usage(&self) -> &str {
        "This command is meant for debugging purposes.\nIt shows you the process information and system memory information."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("debug info")
            .input_output_types(vec![(Type::Nothing, Type::Record(vec![]))])
            .category(Category::Debug)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "View process information",
            example: "debug info",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = Span::unknown();
        // get the nushell process id
        let pid = Pid::from(std::process::id() as usize);
        // only refresh the process and memory information
        let rk = RefreshKind::new()
            .with_processes(
                ProcessRefreshKind::new()
                    .without_cpu()
                    .without_disk_usage()
                    .without_user(),
            )
            .with_memory();
        // only get information requested
        let system = System::new_with_specifics(rk);
        // get the process information for the nushell pid
        let pinfo = system.process(pid);

        if let Some(p) = pinfo {
            Ok(Value::record(
                record! {
                    "pid" => Value::int(p.pid().as_u32() as i64, span),
                    "ppid" => Value::int(p.parent().unwrap_or(0.into()).as_u32() as i64, span),
                    "process" => {
                        Value::record(
                            record! {
                                "memory" => Value::filesize(p.memory() as i64, span),
                                "virtual_memory" => Value::filesize(p.virtual_memory() as i64, span),
                                "status" => Value::string(p.status().to_string(), span),
                                // This is a hack to get the "root" since p.root() doesn't work on macos
                                // Would probably puke if nu was on the root of a drive, maybe other ways too.
                                "root" => {
                                    if let Some(filename) = p.exe().parent() {
                                        Value::string(filename.to_string_lossy().to_string(), span)
                                    } else {
                                        Value::nothing(span)
                                    }
                                },
                                // "root" => Value::string(p.root().to_string_lossy().to_string(), span),
                                "cwd" => Value::string(p.cwd().to_string_lossy().to_string(), span),
                                "exe_path" => Value::string(p.exe().to_string_lossy().to_string(), span),
                                "command" => Value::string(p.cmd().join(" "), span),
                                "name" => Value::string(p.name().to_string(), span),
                                "environment" => {
                                    let mut env_rec = Record::new();
                                    for val in p.environ() {
                                        let (key, value) = val.split_once('=').unwrap_or(("", ""));
                                        // Let's make some of the known list-variables into lists
                                        if key == "PATH" ||
                                           key == "Path" ||
                                           key == "DYLD_FALLBACK_LIBRARY_PATH" ||
                                           key == "PATHEXT" ||
                                           key == "PSMODULEPATH" ||
                                           key == "PSModulePath" {
                                            let items = value.split(ENV_PATH_SEPARATOR_CHAR).map(|r| Value::string(r.to_string(), span)).collect::<Vec<_>>();
                                            env_rec.push(key.to_string(), Value::list(items, span));
                                        } else if key == "LS_COLORS" { // LS_COLORS is a special case, it's a colon separated list of key=value pairs
                                            let items = value.split(':').map(|r| Value::string(r.to_string(), span)).collect::<Vec<_>>();
                                            env_rec.push(key.to_string(), Value::list(items, span));
                                        } else {
                                            env_rec.push(key.to_string(), Value::string(value.to_string(), span));
                                        }
                                    }
                                    Value::record(env_rec, span)
                                },
                            },
                            span,
                        )
                    },
                    "system" => {
                        Value::record(
                            record! {
                                "total_memory" => Value::filesize(system.total_memory() as i64, span),
                                "free_memory" => Value::filesize(system.free_memory() as i64, span),
                                "used_memory" => Value::filesize(system.used_memory() as i64, span),
                                "available_memory" => Value::filesize(system.available_memory() as i64, span),
                            },
                            span,
                        )
                    }
                },
                span,
            ).into_pipeline_data())
        } else {
            // If we can't get the process information, just return the system information
            Ok(Value::record(
                record! {
                    "total_memory" => Value::filesize(system.total_memory() as i64, span),
                    "free_memory" => Value::filesize(system.free_memory() as i64, span),
                    "used_memory" => Value::filesize(system.used_memory() as i64, span),
                    "available_memory" => Value::filesize(system.available_memory() as i64, span),
                },
                span,
            )
            .into_pipeline_data())
        }
    }
}
