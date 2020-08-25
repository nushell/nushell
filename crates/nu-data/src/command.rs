use crate::TaggedListBuilder;
use nu_source::Tag;

use nu_protocol::{NamedType, PositionalType, Signature, TaggedDictBuilder, UntaggedValue, Value};

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

pub fn signature_dict(signature: Signature, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();
    let mut sig = TaggedListBuilder::new(&tag);

    for arg in signature.positional.iter() {
        let is_required = matches!(arg.0, PositionalType::Mandatory(_, _));

        sig.push_value(for_spec(arg.0.name(), "argument", is_required, &tag));
    }

    if signature.rest_positional.is_some() {
        let is_required = false;
        sig.push_value(for_spec("rest", "argument", is_required, &tag));
    }

    for (name, ty) in signature.named.iter() {
        match ty.0 {
            NamedType::Mandatory(_, _) => sig.push_value(for_spec(name, "flag", true, &tag)),
            NamedType::Optional(_, _) => sig.push_value(for_spec(name, "flag", false, &tag)),
            NamedType::Switch(_) => sig.push_value(for_spec(name, "switch", false, &tag)),
        }
    }

    sig.into_value()
}
