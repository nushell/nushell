use landlock::{
    ABI, Access, AccessFs, AccessNet, BitFlags, NetPort, PathBeneath, PathFd, RestrictionStatus,
    Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetError, RulesetStatus,
};

use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Landlock;

fn abi(engine_state: &EngineState, stack: &mut Stack, call: &Call) -> Result<ABI, ShellError> {
    let Some(version) = call.get_flag::<i64>(engine_state, stack, "abi")? else {
        return Ok(ABI::V6);
    };

    Ok(match version {
        1 => ABI::V1,
        2 => ABI::V2,
        3 => ABI::V3,
        4 => ABI::V4,
        5 => ABI::V5,
        6 => ABI::V6,

        _ => {
            return Err(ShellError::InvalidValue {
                valid: "value in range 1..6".to_owned(),
                actual: version.to_string(),
                span: call
                    .get_flag_span(stack, "abi")
                    .expect("Presence checked earlier"),
            });
        }
    })
}

fn ports(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    name: &str,
) -> Result<Vec<u16>, ShellError> {
    if !call.has_flag(engine_state, stack, name)? {
        return Ok(vec![]);
    }

    if let Ok(Some(port)) = call.get_flag::<u16>(engine_state, stack, name) {
        return Ok(vec![port]);
    }

    let Ok(Some(ports)) = call.get_flag::<Vec<u16>>(engine_state, stack, name) else {
        return Err(ShellError::IncorrectValue {
            msg: "port number outside of range 1..<65536".to_owned(),
            call_span: call.span(),
            val_span: call
                .get_flag_span(stack, name)
                .expect("Presence checked earlier"),
        });
    };

    Ok(ports)
}

fn restrict(
    abi: ABI,
    fs: Vec<PathBeneath<PathFd>>,
    no_new_privs: bool,
    bind: &[u16],
    connect: &[u16],
) -> Result<RestrictionStatus, RulesetError> {
    let mut ruleset = Ruleset::default()
        // restrict everything
        .handle_access(AccessFs::from_all(abi))?
        .handle_access(AccessNet::from_all(abi))?
        .create()?;

    ruleset = ruleset.set_no_new_privs(no_new_privs);

    for rule in fs {
        ruleset = ruleset.add_rule(rule)?;
    }
    for port in bind {
        ruleset = ruleset.add_rule(NetPort::new(*port, AccessNet::BindTcp))?;
    }
    for port in connect {
        ruleset = ruleset.add_rule(NetPort::new(*port, AccessNet::ConnectTcp))?;
    }

    ruleset.restrict_self()
}

fn list_to_fs_access(list: Value, abi: ABI, call: &Call) -> Result<BitFlags<AccessFs>, ShellError> {
    let mut out = BitFlags::<AccessFs>::empty();
    let list = list.as_list()?;

    for val in list {
        out |= match val.as_str()? {
            "execute" => AccessFs::Execute.into(),
            "write_file" => AccessFs::WriteFile.into(),
            "read_file" => AccessFs::ReadFile.into(),
            "read_dir" => AccessFs::ReadDir.into(),
            "remove_dir" => AccessFs::RemoveDir.into(),
            "remove_file" => AccessFs::RemoveFile.into(),
            "make_char" => AccessFs::MakeChar.into(),
            "make_dir" => AccessFs::MakeDir.into(),
            "make_reg" => AccessFs::MakeReg.into(),
            "make_sock" => AccessFs::MakeSock.into(),
            "make_fifo" => AccessFs::MakeFifo.into(),
            "make_block" => AccessFs::MakeBlock.into(),
            "make_sym" => AccessFs::MakeSym.into(),
            "refer" => AccessFs::Refer.into(),
            "truncate" => AccessFs::Truncate.into(),
            "ioctl" => AccessFs::IoctlDev.into(),

            "read" => AccessFs::from_read(abi),
            "write" => AccessFs::from_write(abi),
            "all" => AccessFs::from_all(abi),

            other => {
                return Err(ShellError::IncorrectValue {
                    msg: format!("Unknown access modifier: `{other}`"),
                    val_span: call.arguments_span(),
                    call_span: call.span(),
                });
            }
        }
    }

    Ok(out)
}

fn record_to_fs_rules(
    r: Record,
    abi: ABI,
    call: &Call,
) -> Result<Vec<PathBeneath<PathFd>>, ShellError> {
    let mut out = Vec::new();

    for (path, list) in r {
        let Ok(fd) = PathFd::new(path) else {
            return Err(ShellError::GenericError {
                error: "Failed to open a path".to_owned(),
                msg: "Failed to open a path".to_owned(),
                span: Some(call.span()),
                help: None,
                inner: Vec::new(),
            });
        };

        let access = list_to_fs_access(list, abi, call)?;

        let rule = PathBeneath::new(fd, access);

        out.push(rule)
    }

    Ok(out)
}

fn status_to_nu_value(status: RestrictionStatus, span: Span, no_new_privs: bool) -> Value {
    let mut out = Record::new();

    let ruleset_status = match status.ruleset {
        RulesetStatus::FullyEnforced => Value::bool(true, span),
        RulesetStatus::PartiallyEnforced => Value::string("partially", span),
        RulesetStatus::NotEnforced => Value::bool(false, span),
    };
    out.insert("enforced", ruleset_status);
    out.insert("no_new_privs", Value::bool(no_new_privs, span));

    Value::record(out, span)
}

fn one_or_many_ss(shape: SyntaxShape) -> SyntaxShape {
    SyntaxShape::OneOf(vec![shape.clone(), SyntaxShape::List(Box::new(shape))])
}

impl Command for Landlock {
    fn name(&self) -> &str {
        "self landlock"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Experimental)
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .named(
                "abi",
                SyntaxShape::Number,
                "Landlock ABI version to use.  6 by default",
                None,
            )
            .switch("allow-new-privs", "Don't set NO_NEW_PRIVS", None)
            .named(
                "bind-tcp",
                one_or_many_ss(SyntaxShape::Number),
                "Allow binding these TCP ports",
                None,
            )
            .named(
                "connect-tcp",
                one_or_many_ss(SyntaxShape::Number),
                "Allow connecting to these TCP ports",
                None,
            )
            .optional("rules", SyntaxShape::Record(vec![]), "Landlock rules")
    }

    fn description(&self) -> &str {
        "Apply landlock restrictions to the current Nu process"
    }

    fn examples(&self) -> Vec<Example> {
        // TODO
        vec![]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let abi = abi(engine_state, stack, call)?;
        let bind = ports(engine_state, stack, call, "bind-tcp")?;
        let connect = ports(engine_state, stack, call, "connect-tcp")?;
        let no_new_privs = !call.has_flag(engine_state, stack, "allow-new-privs")?;
        let fs_rules = match call.opt::<Record>(engine_state, stack, 0)? {
            Some(rules) => record_to_fs_rules(rules, abi, call)?,
            None => Vec::new(),
        };

        let Ok(status) = restrict(abi, fs_rules, no_new_privs, &bind, &connect) else {
            return Err(ShellError::GenericError {
                msg: "Landlock failed".to_owned(),
                error: "Failed to apply rules".to_owned(),
                span: Some(call.span()),
                help: None,
                inner: Vec::new(),
            });
        };

        Ok(status_to_nu_value(status, call.span(), no_new_privs).into_pipeline_data())
    }
}
