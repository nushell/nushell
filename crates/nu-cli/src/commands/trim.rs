use crate::commands::WholeStreamCommand;

use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Primitive, ReturnSuccess, Signature, UntaggedValue, Value};

pub struct Trim;

impl WholeStreamCommand for Trim {
    fn name(&self) -> &str {
        "trim"
    }

    fn signature(&self) -> Signature {
        Signature::build("trim")
    }

    fn usage(&self) -> &str {
        "Trim leading and following whitespace from text data."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        trim(args, registry)
    }
}

fn trim_primitive(p: &mut Primitive) {
    match p {
        Primitive::String(s) | Primitive::Line(s) => *p = Primitive::String(s.trim().to_string()),
        Primitive::Nothing
        | Primitive::Int(_)
        | Primitive::Decimal(_)
        | Primitive::Bytes(_)
        | Primitive::ColumnPath(_)
        | Primitive::Pattern(_)
        | Primitive::Boolean(_)
        | Primitive::Date(_)
        | Primitive::Duration(_)
        | Primitive::Range(_)
        | Primitive::Path(_)
        | Primitive::Binary(_)
        | Primitive::BeginningOfStream
        | Primitive::EndOfStream => (),
    }
}

fn trim_row(d: &mut Dictionary) {
    for (_, mut value) in d.entries.iter_mut() {
        trim_value(&mut value);
    }
}

fn trim_value(v: &mut Value) {
    match &mut v.value {
        UntaggedValue::Primitive(p) => trim_primitive(p),
        UntaggedValue::Row(row) => trim_row(row),
        _ => (),
    };
}

fn trim(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    Ok(args
        .input
        .map(|v| {
            let mut trimmed = v;
            trim_value(&mut trimmed);
            ReturnSuccess::value(trimmed)
        })
        .to_output_stream())
}
