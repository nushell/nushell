use nu_engine::command_prelude::*;
use sysinfo::{MemoryRefreshKind, Pid, ProcessRefreshKind, RefreshKind, System};

const ENV_PATH_SEPARATOR_CHAR: char = {
    #[cfg(target_family = "windows")]
    {
        ';'
    }
    #[cfg(not(target_family = "windows"))]
    {
        ':'
    }
};

#[derive(Clone)]
pub struct DebugInfo;

impl Command for DebugInfo {
    fn name(&self) -> &str {
        "debug info"
    }

    fn description(&self) -> &str {
        "View process memory info."
    }

    fn extra_description(&self) -> &str {
        "This command is meant for debugging purposes.\nIt shows you the process information and system memory information."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("debug info")
            .input_output_types(vec![(Type::Nothing, Type::record())])
            .category(Category::Debug)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(all_columns(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "View process information",
            example: "debug info",
            result: None,
        }]
    }
}

fn all_columns(span: Span) -> Value {
    let rk = RefreshKind::nothing()
        .with_processes(ProcessRefreshKind::everything())
        .with_memory(MemoryRefreshKind::everything());

    // only get information requested
    let sys = System::new_with_specifics(rk);

    let pid = Pid::from(std::process::id() as usize);
    let ppid = {
        sys.process(pid)
            .and_then(|p| p.parent())
            .map(|p| Value::int(p.as_u32().into(), span))
            .unwrap_or(Value::nothing(span))
    };

    let system = Value::record(
        record! {
            "total_memory" => Value::filesize(sys.total_memory() as i64, span),
            "free_memory" => Value::filesize(sys.free_memory() as i64, span),
            "used_memory" => Value::filesize(sys.used_memory() as i64, span),
            "available_memory" => Value::filesize(sys.available_memory() as i64, span),
        },
        span,
    );

    let process = if let Some(p) = sys.process(pid) {
        let root = if let Some(path) = p.exe().and_then(|p| p.parent()) {
            Value::string(path.to_string_lossy().to_string(), span)
        } else {
            Value::nothing(span)
        };

        let cwd = if let Some(path) = p.cwd() {
            Value::string(path.to_string_lossy().to_string(), span)
        } else {
            Value::nothing(span)
        };

        let exe_path = if let Some(path) = p.exe() {
            Value::string(path.to_string_lossy().to_string(), span)
        } else {
            Value::nothing(span)
        };

        let environment = {
            let mut env_rec = Record::new();
            for val in p.environ() {
                if let Some((key, value)) = val.to_string_lossy().split_once('=') {
                    let is_env_var_a_list = {
                        {
                            #[cfg(target_family = "windows")]
                            {
                                key == "Path"
                                    || key == "PATHEXT"
                                    || key == "PSMODULEPATH"
                                    || key == "PSModulePath"
                            }
                            #[cfg(not(target_family = "windows"))]
                            {
                                key == "PATH" || key == "DYLD_FALLBACK_LIBRARY_PATH"
                            }
                        }
                    };
                    if is_env_var_a_list {
                        let items = value
                            .split(ENV_PATH_SEPARATOR_CHAR)
                            .map(|r| Value::string(r.to_string(), span))
                            .collect::<Vec<_>>();
                        env_rec.push(key.to_string(), Value::list(items, span));
                    } else if key == "LS_COLORS" {
                        // LS_COLORS is a special case, it's a colon separated list of key=value pairs
                        let items = value
                            .split(':')
                            .map(|r| Value::string(r.to_string(), span))
                            .collect::<Vec<_>>();
                        env_rec.push(key.to_string(), Value::list(items, span));
                    } else {
                        env_rec.push(key.to_string(), Value::string(value.to_string(), span));
                    }
                }
            }
            Value::record(env_rec, span)
        };

        Value::record(
            record! {
                "memory" => Value::filesize(p.memory() as i64, span),
                "virtual_memory" => Value::filesize(p.virtual_memory() as i64, span),
                "status" => Value::string(p.status().to_string(), span),
                "root" => root,
                "cwd" => cwd,
                "exe_path" => exe_path,
                "command" => Value::string(p.cmd().join(std::ffi::OsStr::new(" ")).to_string_lossy(), span),
                "name" => Value::string(p.name().to_string_lossy(), span),
                "environment" => environment,
            },
            span,
        )
    } else {
        Value::nothing(span)
    };

    Value::record(
        record! {
            "thread_id" => Value::int(get_thread_id() as i64, span),
            "pid" => Value::int(pid.as_u32().into(), span),
            "ppid" => ppid,
            "system" => system,
            "process" => process,
        },
        span,
    )
}

fn get_thread_id() -> u64 {
    #[cfg(windows)]
    {
        unsafe { windows::Win32::System::Threading::GetCurrentThreadId().into() }
    }
    #[cfg(unix)]
    {
        nix::sys::pthread::pthread_self() as u64
    }
    #[cfg(target_arch = "wasm32")]
    {
        // wasm doesn't have any threads accessible, so we return 0 as a fallback
        0
    }
}
