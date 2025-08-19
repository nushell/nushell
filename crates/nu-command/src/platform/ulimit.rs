use nix::sys::resource::{RLIM_INFINITY, Resource, rlim_t};
use nu_engine::command_prelude::*;

use std::sync::LazyLock;

/// An object contains resource related parameters
struct ResourceInfo<'a> {
    name: &'a str,
    desc: &'a str,
    flag: char,
    multiplier: rlim_t,
    resource: Resource,
}

impl<'a> ResourceInfo<'a> {
    /// Create a `ResourceInfo` object
    fn new(
        name: &'a str,
        desc: &'a str,
        flag: char,
        multiplier: rlim_t,
        resource: Resource,
    ) -> Self {
        Self {
            name,
            desc,
            flag,
            multiplier,
            resource,
        }
    }

    /// Get unit
    fn get_unit(&self) -> &str {
        if self.resource == Resource::RLIMIT_CPU {
            "(seconds, "
        } else if self.multiplier == 1 {
            "("
        } else {
            "(kB, "
        }
    }
}

impl Default for ResourceInfo<'_> {
    fn default() -> Self {
        Self {
            name: "file-size",
            desc: "Maximum size of files created by the shell",
            flag: 'f',
            multiplier: 1024,
            resource: Resource::RLIMIT_FSIZE,
        }
    }
}

static RESOURCE_ARRAY: LazyLock<Vec<ResourceInfo>> = LazyLock::new(|| {
    let resources = [
        #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
        (
            "socket-buffers",
            "Maximum size of socket buffers",
            'b',
            1024,
            Resource::RLIMIT_SBSIZE,
        ),
        (
            "core-size",
            "Maximum size of core files created",
            'c',
            1024,
            Resource::RLIMIT_CORE,
        ),
        (
            "data-size",
            "Maximum size of a process's data segment",
            'd',
            1024,
            Resource::RLIMIT_DATA,
        ),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        (
            "nice",
            "Controls of maximum nice priority",
            'e',
            1,
            Resource::RLIMIT_NICE,
        ),
        (
            "file-size",
            "Maximum size of files created by the shell",
            'f',
            1024,
            Resource::RLIMIT_FSIZE,
        ),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        (
            "pending-signals",
            "Maximum number of pending signals",
            'i',
            1,
            Resource::RLIMIT_SIGPENDING,
        ),
        #[cfg(any(
            target_os = "android",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "netbsd"
        ))]
        (
            "lock-size",
            "Maximum size that may be locked into memory",
            'l',
            1024,
            Resource::RLIMIT_MEMLOCK,
        ),
        #[cfg(any(
            target_os = "android",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "aix",
        ))]
        (
            "resident-set-size",
            "Maximum resident set size",
            'm',
            1024,
            Resource::RLIMIT_RSS,
        ),
        (
            "file-descriptor-count",
            "Maximum number of open file descriptors",
            'n',
            1,
            Resource::RLIMIT_NOFILE,
        ),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        (
            "queue-size",
            "Maximum bytes in POSIX message queues",
            'q',
            1024,
            Resource::RLIMIT_MSGQUEUE,
        ),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        (
            "realtime-priority",
            "Maximum realtime scheduling priority",
            'r',
            1,
            Resource::RLIMIT_RTPRIO,
        ),
        (
            "stack-size",
            "Maximum stack size",
            's',
            1024,
            Resource::RLIMIT_STACK,
        ),
        (
            "cpu-time",
            "Maximum amount of CPU time in seconds",
            't',
            1,
            Resource::RLIMIT_CPU,
        ),
        #[cfg(any(
            target_os = "android",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "aix",
        ))]
        (
            "process-count",
            "Maximum number of processes available to the current user",
            'u',
            1,
            Resource::RLIMIT_NPROC,
        ),
        #[cfg(not(any(target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")))]
        (
            "virtual-memory-size",
            "Maximum amount of virtual memory available to each process",
            'v',
            1024,
            Resource::RLIMIT_AS,
        ),
        #[cfg(target_os = "freebsd")]
        (
            "swap-size",
            "Maximum swap space",
            'w',
            1024,
            Resource::RLIMIT_SWAP,
        ),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        (
            "file-locks",
            "Maximum number of file locks",
            'x',
            1,
            Resource::RLIMIT_LOCKS,
        ),
        #[cfg(target_os = "linux")]
        (
            "realtime-maxtime",
            "Maximum contiguous realtime CPU time",
            'y',
            1,
            Resource::RLIMIT_RTTIME,
        ),
        #[cfg(target_os = "freebsd")]
        (
            "kernel-queues",
            "Maximum number of kqueues",
            'K',
            1,
            Resource::RLIMIT_KQUEUES,
        ),
        #[cfg(target_os = "freebsd")]
        (
            "ptys",
            "Maximum number of pseudo-terminals",
            'P',
            1,
            Resource::RLIMIT_NPTS,
        ),
    ];

    let mut resource_array = Vec::new();
    for (name, desc, flag, multiplier, res) in resources {
        resource_array.push(ResourceInfo::new(name, desc, flag, multiplier, res));
    }

    resource_array
});

/// Convert `rlim_t` to `Value` representation
fn limit_to_value(limit: rlim_t, multiplier: rlim_t, span: Span) -> Result<Value, ShellError> {
    if limit == RLIM_INFINITY {
        return Ok(Value::string("unlimited", span));
    }

    let val = match i64::try_from(limit / multiplier) {
        Ok(v) => v,
        Err(e) => {
            return Err(ShellError::CantConvert {
                to_type: "i64".into(),
                from_type: "rlim_t".into(),
                span,
                help: Some(e.to_string()),
            });
        }
    };

    Ok(Value::int(val, span))
}

/// Get maximum length of all flag descriptions
fn max_desc_len(
    call: &Call,
    engine_state: &EngineState,
    stack: &mut Stack,
    print_all: bool,
) -> Result<usize, ShellError> {
    let mut desc_len = 0;
    let mut unit_len = 0;

    for res in RESOURCE_ARRAY.iter() {
        if !print_all && !call.has_flag(engine_state, stack, res.name)? {
            continue;
        }

        desc_len = res.desc.len().max(desc_len);
        unit_len = res.get_unit().len().max(unit_len);
    }

    // Use `RLIMIT_FSIZE` limit if no resource flag provided.
    if desc_len == 0 {
        let res = ResourceInfo::default();
        desc_len = res.desc.len().max(desc_len);
        unit_len = res.get_unit().len().max(unit_len);
    }

    // desc.len() + unit.len() + '-X)'.len()
    Ok(desc_len + unit_len + 3)
}

/// Fill `ResourceInfo` to the record entry
fn fill_record(
    res: &ResourceInfo,
    max_len: usize,
    soft: bool,
    hard: bool,
    span: Span,
) -> Result<Record, ShellError> {
    let mut record = Record::new();
    let mut desc = String::new();

    desc.push_str(res.desc);

    debug_assert!(res.desc.len() + res.get_unit().len() + 3 <= max_len);
    let width = max_len - res.desc.len() - res.get_unit().len() - 3;
    if width == 0 {
        desc.push_str(format!(" {}-{})", res.get_unit(), res.flag).as_str());
    } else {
        desc.push_str(format!("{:>width$} {}-{})", ' ', res.get_unit(), res.flag).as_str());
    }

    record.push("description", Value::string(desc, span));

    let (soft_limit, hard_limit) = getrlimit(res.resource)?;

    if soft {
        let soft_limit = limit_to_value(soft_limit, res.multiplier, span)?;
        record.push("soft", soft_limit);
    }

    if hard {
        let hard_limit = limit_to_value(hard_limit, res.multiplier, span)?;
        record.push("hard", hard_limit);
    }

    Ok(record)
}

/// Set limits
fn set_limits(
    limit_value: &Value,
    res: &ResourceInfo,
    soft: bool,
    hard: bool,
    call_span: Span,
) -> Result<(), ShellError> {
    let (mut soft_limit, mut hard_limit) = getrlimit(res.resource)?;
    let new_limit = parse_limit(limit_value, res, soft, soft_limit, hard_limit, call_span)?;

    if hard {
        hard_limit = new_limit;
    }

    if soft {
        soft_limit = new_limit;

        // Do not attempt to set the soft limit higher than the hard limit.
        if (new_limit > hard_limit || new_limit == RLIM_INFINITY) && hard_limit != RLIM_INFINITY {
            soft_limit = hard_limit;
        }
    }

    setrlimit(res.resource, soft_limit, hard_limit)
}

/// Print limits
fn print_limits(
    call: &Call,
    engine_state: &EngineState,
    stack: &mut Stack,
    print_all: bool,
    soft: bool,
    hard: bool,
) -> Result<PipelineData, ShellError> {
    let mut output = Vec::new();
    let mut print_default_limit = true;
    let max_len = max_desc_len(call, engine_state, stack, print_all)?;

    for res in RESOURCE_ARRAY.iter() {
        if !print_all {
            // Print specified limit.
            if !call.has_flag(engine_state, stack, res.name)? {
                continue;
            }
        }

        let record = fill_record(res, max_len, soft, hard, call.head)?;
        output.push(Value::record(record, call.head));

        if print_default_limit {
            print_default_limit = false;
        }
    }

    // Print `RLIMIT_FSIZE` limit if no resource flag provided.
    if print_default_limit {
        let res = ResourceInfo::default();
        let record = fill_record(&res, max_len, soft, hard, call.head)?;
        output.push(Value::record(record, call.head));
    }

    Ok(Value::list(output, call.head).into_pipeline_data())
}

/// Wrap `nix::sys::resource::getrlimit`
fn setrlimit(res: Resource, soft_limit: rlim_t, hard_limit: rlim_t) -> Result<(), ShellError> {
    nix::sys::resource::setrlimit(res, soft_limit, hard_limit).map_err(|e| {
        ShellError::GenericError {
            error: e.to_string(),
            msg: String::new(),
            span: None,
            help: None,
            inner: vec![],
        }
    })
}

/// Wrap `nix::sys::resource::setrlimit`
fn getrlimit(res: Resource) -> Result<(rlim_t, rlim_t), ShellError> {
    nix::sys::resource::getrlimit(res).map_err(|e| ShellError::GenericError {
        error: e.to_string(),
        msg: String::new(),
        span: None,
        help: None,
        inner: vec![],
    })
}

/// Parse user input
fn parse_limit(
    limit_value: &Value,
    res: &ResourceInfo,
    soft: bool,
    soft_limit: rlim_t,
    hard_limit: rlim_t,
    call_span: Span,
) -> Result<rlim_t, ShellError> {
    let val_span = limit_value.span();
    match limit_value {
        Value::Int { val, .. } => {
            let value = rlim_t::try_from(*val).map_err(|e| ShellError::CantConvert {
                to_type: "rlim_t".into(),
                from_type: "i64".into(),
                span: val_span,
                help: Some(e.to_string()),
            })?;

            let (limit, overflow) = value.overflowing_mul(res.multiplier);
            if overflow {
                Ok(RLIM_INFINITY)
            } else {
                Ok(limit)
            }
        }
        Value::Filesize { val, .. } => {
            if res.multiplier != 1024 {
                return Err(ShellError::TypeMismatch {
                    err_message: format!(
                        "filesize is not compatible with resource {:?}",
                        res.resource
                    ),
                    span: val_span,
                });
            }

            rlim_t::try_from(*val).map_err(|e| ShellError::CantConvert {
                to_type: "rlim_t".into(),
                from_type: "i64".into(),
                span: val_span,
                help: Some(e.to_string()),
            })
        }
        Value::String { val, .. } => {
            if val == "unlimited" {
                Ok(RLIM_INFINITY)
            } else if val == "soft" {
                if soft { Ok(hard_limit) } else { Ok(soft_limit) }
            } else if val == "hard" {
                Ok(hard_limit)
            } else {
                Err(ShellError::IncorrectValue {
                    msg: "Only unlimited, soft and hard are supported for strings".into(),
                    val_span,
                    call_span,
                })
            }
        }
        _ => Err(ShellError::TypeMismatch {
            err_message: format!(
                "string, int or filesize required, you provide {}",
                limit_value.get_type()
            ),
            span: limit_value.span(),
        }),
    }
}

#[derive(Clone)]
pub struct ULimit;

impl Command for ULimit {
    fn name(&self) -> &str {
        "ulimit"
    }

    fn description(&self) -> &str {
        "Set or get resource usage limits."
    }

    fn signature(&self) -> Signature {
        let mut sig = Signature::build("ulimit")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .switch("soft", "Sets soft resource limit", Some('S'))
            .switch("hard", "Sets hard resource limit", Some('H'))
            .switch("all", "Prints all current limits", Some('a'))
            .optional("limit", SyntaxShape::Any, "Limit value.")
            .category(Category::Platform);

        for res in RESOURCE_ARRAY.iter() {
            sig = sig.switch(res.name, res.desc, Some(res.flag));
        }

        sig
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut soft = call.has_flag(engine_state, stack, "soft")?;
        let mut hard = call.has_flag(engine_state, stack, "hard")?;
        let all = call.has_flag(engine_state, stack, "all")?;

        if !hard && !soft {
            // Set both hard and soft limits if neither was specified.
            hard = true;
            soft = true;
        }

        if let Some(limit_value) = call.opt::<Value>(engine_state, stack, 0)? {
            let mut set_default_limit = true;

            for res in RESOURCE_ARRAY.iter() {
                if call.has_flag(engine_state, stack, res.name)? {
                    set_limits(&limit_value, res, soft, hard, call.head)?;

                    if set_default_limit {
                        set_default_limit = false;
                    }
                }
            }

            // Set `RLIMIT_FSIZE` limit if no resource flag provided.
            if set_default_limit {
                let res = ResourceInfo::default();
                set_limits(&limit_value, &res, hard, soft, call.head)?;
            }

            Ok(PipelineData::empty())
        } else {
            print_limits(call, engine_state, stack, all, soft, hard)
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Print all current limits",
                example: "ulimit -a",
                result: None,
            },
            Example {
                description: "Print specified limits",
                example: "ulimit --core-size --data-size --file-size",
                result: None,
            },
            Example {
                description: "Set limit",
                example: "ulimit --core-size 102400",
                result: None,
            },
            Example {
                description: "Set stack size soft limit",
                example: "ulimit -s -S 10240",
                result: None,
            },
            Example {
                description: "Set virtual memory size hard limit",
                example: "ulimit -v -H 10240",
                result: None,
            },
            Example {
                description: "Set core size limit to unlimited",
                example: "ulimit -c unlimited",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["resource", "limits"]
    }
}
