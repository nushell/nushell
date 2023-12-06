use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};

use nix::sys::resource::{rlim_t, Resource, RLIM_INFINITY};

/// Limit type
fn limit_type(name: &str) -> Type {
    match name {
        "socket-buffers"
        | "core-size"
        | "data-size"
        | "file-size"
        | "lock-size"
        | "resident-set-size"
        | "queue-size"
        | "stack-size"
        | "virtual-memory-size"
        | "swap-size" => Type::Filesize,
        _ => Type::Int,
    }
}

/// Convert `rlim_t` to `Value` representation
fn limit_to_value(limit: rlim_t, name: &str, span: Span) -> Result<Value, ShellError> {
    if limit == RLIM_INFINITY {
        return Ok(Value::string("unlimited", span));
    }

    let typ = limit_type(name);

    let val = match i64::try_from(limit) {
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

    match typ {
        Type::Filesize => Ok(Value::filesize(val, span)),
        _ => Ok(Value::int(val, span)),
    }
}

/// Get maximum length of all flag descriptions
fn max_desc_len(sig: &Signature) -> usize {
    let mut len = 0;
    for name in sig.get_names().iter() {
        let Some(ref flag) = sig.get_long_flag(name) else {
            continue;
        };

        len = flag.desc.len().max(len);
    }

    len
}

/// Fill the record entry
fn fill_record(
    record: &mut Record,
    name: &str,
    len: usize,
    hard: bool,
    res: Resource,
    sig: &Signature,
    span: Span,
) -> Result<(), ShellError> {
    let (soft_limit, hard_limit) = getrlimit(res)?;

    let limit = if hard {
        limit_to_value(hard_limit, name, span)?
    } else {
        limit_to_value(soft_limit, name, span)?
    };

    let mut col = String::new();
    match sig.get_long_flag(name) {
        Some(ref flag) => {
            col.push_str(flag.desc.as_str());

            if let Some(char) = flag.short {
                let width = len - flag.desc.len();
                if width == 0 {
                    col.push_str(format!(" (-{})", char).as_str());
                } else {
                    col.push_str(format!("{:>width$} (-{})", ' ', char).as_str());
                }
            }
        }
        None => col.push_str(name),
    }

    record.push(col, limit);

    Ok(())
}

/// Show limits
fn show_limits(
    call: &Call,
    sig: &Signature,
    all: bool,
    hard: bool,
) -> Result<PipelineData, ShellError> {
    let mut record = Record::new();
    let mut show_default_limit = true;
    let len = max_desc_len(sig);

    for name in sig.get_names().iter() {
        if !all {
            // Show specified limit.
            if !call.has_flag(name) {
                continue;
            }
        }

        let Some(res) = str_to_resource(name) else {
            continue;
        };

        fill_record(&mut record, name, len, hard, res, sig, call.head)?;

        show_default_limit = false;
    }

    // Show `RLIMIT_FSIZE` limit if no flag provided
    if show_default_limit {
        fill_record(
            &mut record,
            "file-size",
            len,
            hard,
            Resource::RLIMIT_FSIZE,
            sig,
            call.head,
        )?;
    }

    Ok(Value::record(record, call.head).into_pipeline_data())
}

/// Convert `&str` to `Option<Resource>`
fn str_to_resource(s: &str) -> Option<Resource> {
    match s {
        #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
        "socket-buffers" => Some(Resource::RLIMIT_SBSIZE),
        "core-size" => Some(Resource::RLIMIT_CORE),
        "data-size" => Some(Resource::RLIMIT_DATA),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        "nice" => Some(Resource::RLIMIT_NICE),
        "file-size" => Some(Resource::RLIMIT_FSIZE),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        "pending-signals" => Some(Resource::RLIMIT_SIGPENDING),
        #[cfg(any(
            target_os = "android",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "linux",
            target_os = "netbsd"
        ))]
        "lock-size" => Some(Resource::RLIMIT_MEMLOCK),
        #[cfg(any(
            target_os = "android",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "linux",
            target_os = "aix",
        ))]
        "resident-set-size" => Some(Resource::RLIMIT_RSS),
        "file-descriptor-count" => Some(Resource::RLIMIT_NOFILE),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        "queue-size" => Some(Resource::RLIMIT_MSGQUEUE),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        "realtime-priority" => Some(Resource::RLIMIT_RTPRIO),
        "stack-size" => Some(Resource::RLIMIT_STACK),
        "cpu-time" => Some(Resource::RLIMIT_CPU),
        #[cfg(any(
            target_os = "android",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "linux",
            target_os = "aix",
        ))]
        "process-count" => Some(Resource::RLIMIT_NPROC),
        #[cfg(not(any(target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")))]
        "virtual-memory-size" => Some(Resource::RLIMIT_AS),
        #[cfg(target_os = "freebsd")]
        "swap-size" => Some(Resource::RLIMIT_SWAP),
        #[cfg(any(target_os = "android", target_os = "linux"))]
        "file-locks" => Some(Resource::RLIMIT_LOCKS),
        #[cfg(target_os = "linux")]
        "realtime-maxtime" => Some(Resource::RLIMIT_RTTIME),
        #[cfg(target_os = "freebsd")]
        "kernel-queues" => Some(Resource::RLIMIT_KQUEUES),
        #[cfg(target_os = "freebsd")]
        "ptys" => Some(Resource::RLIMIT_NPTY),
        _ => None,
    }
}

/// Wrap `nix::sys::resource::getrlimit`
fn setrlimit(res: Resource, soft_limit: rlim_t, hard_limit: rlim_t) -> Result<(), ShellError> {
    nix::sys::resource::setrlimit(res, soft_limit, hard_limit)
        .map_err(|e| ShellError::GenericError(e.to_string(), String::new(), None, None, vec![]))
}

/// Wrap `nix::sys::resource::setrlimit`
fn getrlimit(res: Resource) -> Result<(rlim_t, rlim_t), ShellError> {
    nix::sys::resource::getrlimit(res)
        .map_err(|e| ShellError::GenericError(e.to_string(), String::new(), None, None, vec![]))
}

/// Parse user input
fn parse_limit(spanned_limit: &Spanned<String>) -> Result<rlim_t, ShellError> {
    let limit = &spanned_limit.item;
    let span = spanned_limit.span;

    if limit.eq("unlimited") {
        Ok(RLIM_INFINITY)
    } else {
        limit
            .parse::<rlim_t>()
            .map_err(|e| ShellError::CantConvert {
                to_type: "rlim_t".into(),
                from_type: "String".into(),
                span,
                help: Some(e.to_string()),
            })
    }
}

#[derive(Clone)]
pub struct ULimit;

impl Command for ULimit {
    fn name(&self) -> &str {
        "ulimit"
    }

    fn usage(&self) -> &str {
        "Set or get resource usage limits"
    }

    fn signature(&self) -> Signature {
        let sig = Signature::build("ulimit")
            .input_output_type(Type::Nothing, Type::Record(vec![]))
            .switch("soft", "Sets soft resource limit", Some('S'))
            .switch("hard", "Sets hard resource limit", Some('H'))
            .switch("all", "Prints all current limits", Some('a'))
            .optional("limit", SyntaxShape::String, "Limit value")
            .category(Category::Platform);

        #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
        let sig = sig.switch(
            "socket-buffers",
            "Maximum size of socket buffers",
            Some('b'),
        );

        let sig = sig
            .switch("core-size", "Maximum size of core files created", Some('c'))
            .switch(
                "data-size",
                "Maximum size of a process's data segment",
                Some('d'),
            );

        #[cfg(any(target_os = "android", target_os = "linux"))]
        let sig = sig.switch("nice", "Controls of maximum nice priority", Some('e'));

        let sig = sig.switch(
            "file-size",
            "Maximum size of files created by the shell",
            Some('f'),
        );

        #[cfg(any(target_os = "android", target_os = "linux"))]
        let sig = sig.switch(
            "pending-signals",
            "Maximum number of pending signals",
            Some('i'),
        );

        #[cfg(any(
            target_os = "android",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "linux",
            target_os = "netbsd"
        ))]
        let sig = sig.switch(
            "lock-size",
            "Maximum size that may be locked into memory",
            Some('l'),
        );

        #[cfg(any(
            target_os = "android",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "linux",
            target_os = "aix",
        ))]
        let sig = sig.switch("resident-set-size", "Maximum resident set size", Some('m'));

        let sig = sig.switch(
            "file-descriptor-count",
            "Maximum number of open file descriptors",
            Some('n'),
        );

        #[cfg(any(target_os = "android", target_os = "linux"))]
        let sig = sig.switch(
            "queue-size",
            "Maximum bytes in POSIX message queues",
            Some('q'),
        );

        #[cfg(any(target_os = "android", target_os = "linux"))]
        let sig = sig.switch(
            "realtime-priority",
            "Maximum realtime scheduling priority",
            Some('r'),
        );

        let sig = sig
            .switch("stack-size", "Maximum stack size", Some('s'))
            .switch(
                "cpu-time",
                "Maximum amount of CPU time in seconds",
                Some('t'),
            );

        #[cfg(any(
            target_os = "android",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "linux",
            target_os = "aix",
        ))]
        let sig = sig.switch(
            "process-count",
            "Maximum number of processes available to the current user",
            Some('u'),
        );

        #[cfg(not(any(target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")))]
        let sig = sig.switch(
            "virtual-memory-size",
            "Maximum amount of virtual memory available to each process",
            Some('v'),
        );

        #[cfg(target_os = "freebsd")]
        let sig = sig.switch("swap-size", "Maximum swap space", Some('w'));

        #[cfg(any(target_os = "android", target_os = "linux"))]
        let sig = sig.switch("file-locks", "Maximum number of file locks", Some('x'));

        #[cfg(target_os = "linux")]
        let sig = sig.switch(
            "realtime-maxtime",
            "Maximum contiguous realtime CPU time",
            Some('y'),
        );

        #[cfg(target_os = "freebsd")]
        let sig = sig.switch("kernel-queues", "Maximum number of kqueues", Some('K'));

        #[cfg(target_os = "freebsd")]
        let sig = sig.switch("ptys", "Maximum number of pseudo-terminals", Some('P'));

        sig
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut soft = call.has_flag("soft");
        let mut hard = call.has_flag("hard");

        if let Some(spanned_limit) = call.opt::<Spanned<String>>(engine_state, stack, 0)? {
            if !hard && !soft {
                // Set both hard and soft limits if neither was specified.
                hard = true;
                soft = true;
            }

            let value = parse_limit(&spanned_limit)?;

            for name in self.signature().get_names() {
                if call.has_flag(name) {
                    let Some(res) = str_to_resource(name) else {
                        continue;
                    };

                    let (mut soft_limit, mut hard_limit) = getrlimit(res)?;

                    if hard {
                        hard_limit = value;
                    }

                    if soft {
                        soft_limit = value;

                        // Do not attempt to set the soft limit higher than the ahard limit.
                        if (value > hard_limit || value == RLIM_INFINITY)
                            && hard_limit != RLIM_INFINITY
                        {
                            soft_limit = hard_limit;
                        }
                    }

                    setrlimit(res, soft_limit, hard_limit)?;
                }
            }

            Ok(PipelineData::Empty)
        } else {
            let sig = self.signature();

            if call.has_flag("all") {
                show_limits(call, &sig, true, hard)
            } else {
                show_limits(call, &sig, false, hard)
            }
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
                example: "ulimit -c -d -f",
                result: None,
            },
            Example {
                description: "Set limit",
                example: "ulimit --stack-size 102400",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["resource", "limits"]
    }
}
