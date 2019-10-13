use crate::commands::command::Command;
use crate::data::{TaggedDictBuilder, TaggedListBuilder, Value};
use crate::parser::registry::{NamedType, PositionalType, Signature};
use crate::prelude::*;
use std::ops::Deref;

pub(crate) fn command_dict(command: Arc<Command>, tag: impl Into<Tag>) -> Tagged<Value> {
    let tag = tag.into();

    let mut cmd_dict = TaggedDictBuilder::new(&tag);

    cmd_dict.insert("name", Value::string(command.name()));

    cmd_dict.insert(
        "type",
        Value::string(match command.deref() {
            Command::WholeStream(_) => "Command",
            Command::PerItem(_) => "Filter",
        }),
    );

    cmd_dict.insert_tagged("signature", signature_dict(command.signature(), tag));
    cmd_dict.insert("usage", Value::string(command.usage()));

    cmd_dict.into_tagged_value()
}

fn for_spec(name: &str, ty: &str, required: bool, tag: impl Into<Tag>) -> Tagged<Value> {
    let tag = tag.into();

    let mut spec = TaggedDictBuilder::new(tag);

    spec.insert("name", Value::string(name));
    spec.insert("type", Value::string(ty));
    spec.insert(
        "required",
        Value::string(if required { "yes" } else { "no" }),
    );

    spec.into_tagged_value()
}

fn signature_dict(signature: Signature, tag: impl Into<Tag>) -> Tagged<Value> {
    let tag = tag.into();
    let mut sig = TaggedListBuilder::new(&tag);

    for arg in signature.positional.iter() {
        let is_required = match arg {
            PositionalType::Mandatory(_, _) => true,
            PositionalType::Optional(_, _) => false,
        };

        sig.insert_tagged(for_spec(arg.name(), "argument", is_required, &tag));
    }

    if let Some(_) = signature.rest_positional {
        let is_required = false;
        sig.insert_tagged(for_spec("rest", "argument", is_required, &tag));
    }

    for (name, ty) in signature.named.iter() {
        match ty {
            NamedType::Mandatory(_) => sig.insert_tagged(for_spec(name, "flag", true, &tag)),
            NamedType::Optional(_) => sig.insert_tagged(for_spec(name, "flag", false, &tag)),
            NamedType::Switch => sig.insert_tagged(for_spec(name, "switch", false, &tag)),
        }
    }

    sig.into_tagged_value()
}
