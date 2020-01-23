use crate::commands::command::Command;
use crate::data::TaggedListBuilder;
use crate::prelude::*;
use nu_protocol::{NamedType, PositionalType, Signature, TaggedDictBuilder, UntaggedValue, Value};
use std::ops::Deref;

pub(crate) fn command_dict(command: Arc<Command>, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    let mut cmd_dict = TaggedDictBuilder::new(&tag);

    cmd_dict.insert_untagged("name", UntaggedValue::string(command.name()));

    cmd_dict.insert_untagged(
        "type",
        UntaggedValue::string(match command.deref() {
            Command::WholeStream(_) => "Command",
            Command::PerItem(_) => "Filter",
        }),
    );

    cmd_dict.insert_value("signature", signature_dict(command.signature(), tag));
    cmd_dict.insert_untagged("usage", UntaggedValue::string(command.usage()));

    cmd_dict.into_value()
}

fn for_spec(name: &str, ty: &str, required: bool, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    let mut spec = TaggedDictBuilder::new(tag);

    spec.insert_untagged("name", UntaggedValue::string(name));
    spec.insert_untagged("type", UntaggedValue::string(ty));
    spec.insert_untagged(
        "required",
        UntaggedValue::string(if required { "yes" } else { "no" }),
    );

    spec.into_value()
}

fn signature_dict(signature: Signature, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();
    let mut sig = TaggedListBuilder::new(&tag);

    for arg in signature.positional.iter() {
        let is_required = match arg.0 {
            PositionalType::Mandatory(_, _) => true,
            PositionalType::Optional(_, _) => false,
        };

        sig.push_value(for_spec(arg.0.name(), "argument", is_required, &tag));
    }

    if signature.rest_positional.is_some() {
        let is_required = false;
        sig.push_value(for_spec("rest", "argument", is_required, &tag));
    }

    for (name, ty) in signature.named.iter() {
        match ty.0 {
            NamedType::Mandatory(_) => sig.push_value(for_spec(name, "flag", true, &tag)),
            NamedType::Optional(_) => sig.push_value(for_spec(name, "flag", false, &tag)),
            NamedType::Switch => sig.push_value(for_spec(name, "switch", false, &tag)),
        }
    }

    sig.into_value()
}
