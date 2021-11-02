use crate::plugin::PluginError;
use crate::plugin_capnp::{argument, flag, option, signature, Shape};
use nu_protocol::{Flag, PositionalArg, Signature, SyntaxShape};

pub(crate) fn serialize_signature(signature: &Signature, mut builder: signature::Builder) {
    builder.set_name(signature.name.as_str());
    builder.set_usage(signature.usage.as_str());
    builder.set_extra_usage(signature.extra_usage.as_str());
    builder.set_is_filter(signature.is_filter);

    // Serializing list of required arguments
    let mut required_list = builder
        .reborrow()
        .init_required_positional(signature.required_positional.len() as u32);

    for (index, arg) in signature.required_positional.iter().enumerate() {
        let inner_builder = required_list.reborrow().get(index as u32);
        serialize_argument(arg, inner_builder)
    }

    // Serializing list of optional arguments
    let mut optional_list = builder
        .reborrow()
        .init_optional_positional(signature.optional_positional.len() as u32);

    for (index, arg) in signature.optional_positional.iter().enumerate() {
        let inner_builder = optional_list.reborrow().get(index as u32);
        serialize_argument(arg, inner_builder)
    }

    // Serializing rest argument
    let mut rest_argument = builder.reborrow().init_rest();
    match &signature.rest_positional {
        None => rest_argument.set_none(()),
        Some(arg) => {
            let inner_builder = rest_argument.init_some();
            serialize_argument(arg, inner_builder)
        }
    }

    // Serializing the named arguments
    let mut named_list = builder.reborrow().init_named(signature.named.len() as u32);

    for (index, arg) in signature.named.iter().enumerate() {
        let inner_builder = named_list.reborrow().get(index as u32);
        serialize_flag(arg, inner_builder)
    }
}

fn serialize_argument(arg: &PositionalArg, mut builder: argument::Builder) {
    builder.set_name(arg.name.as_str());
    builder.set_desc(arg.desc.as_str());

    match arg.shape {
        SyntaxShape::Boolean => builder.set_shape(Shape::Boolean),
        SyntaxShape::String => builder.set_shape(Shape::String),
        SyntaxShape::Int => builder.set_shape(Shape::Int),
        SyntaxShape::Number => builder.set_shape(Shape::Number),
        _ => builder.set_shape(Shape::Any),
    }
}

fn serialize_flag(arg: &Flag, mut builder: flag::Builder) {
    builder.set_long(arg.long.as_str());
    builder.set_required(arg.required);
    builder.set_desc(arg.desc.as_str());

    let mut short_builder = builder.reborrow().init_short();
    match arg.short {
        None => short_builder.set_none(()),
        Some(val) => {
            let mut inner_builder = short_builder.reborrow().initn_some(1);
            inner_builder.push_str(format!("{}", val).as_str());
        }
    }

    match &arg.arg {
        None => builder.set_arg(Shape::None),
        Some(shape) => match shape {
            SyntaxShape::Boolean => builder.set_arg(Shape::Boolean),
            SyntaxShape::String => builder.set_arg(Shape::String),
            SyntaxShape::Int => builder.set_arg(Shape::Int),
            SyntaxShape::Number => builder.set_arg(Shape::Number),
            _ => builder.set_arg(Shape::Any),
        },
    }
}

pub(crate) fn deserialize_signature(reader: signature::Reader) -> Result<Signature, PluginError> {
    let name = reader
        .get_name()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;
    let usage = reader
        .get_usage()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;
    let extra_usage = reader
        .get_extra_usage()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;
    let is_filter = reader.get_is_filter();

    // Deserializing required arguments
    let required_list = reader
        .get_required_positional()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let required_positional = required_list
        .iter()
        .map(deserialize_argument)
        .collect::<Result<Vec<PositionalArg>, PluginError>>()?;

    // Deserializing optional arguments
    let optional_list = reader
        .get_optional_positional()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let optional_positional = optional_list
        .iter()
        .map(deserialize_argument)
        .collect::<Result<Vec<PositionalArg>, PluginError>>()?;

    // Deserializing rest arguments
    let rest_option = reader
        .get_rest()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let rest_positional = match rest_option.which() {
        Err(capnp::NotInSchema(_)) => None,
        Ok(option::None(())) => None,
        Ok(option::Some(rest_reader)) => {
            let rest_reader = rest_reader.map_err(|e| PluginError::EncodingError(e.to_string()))?;
            Some(deserialize_argument(rest_reader)?)
        }
    };

    // Deserializing named arguments
    let named_list = reader
        .get_named()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let named = named_list
        .iter()
        .map(deserialize_flag)
        .collect::<Result<Vec<Flag>, PluginError>>()?;

    Ok(Signature {
        name: name.to_string(),
        usage: usage.to_string(),
        extra_usage: extra_usage.to_string(),
        required_positional,
        optional_positional,
        rest_positional,
        named,
        is_filter,
        creates_scope: false,
    })
}

fn deserialize_argument(reader: argument::Reader) -> Result<PositionalArg, PluginError> {
    let name = reader
        .get_name()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let desc = reader
        .get_desc()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let shape = reader
        .get_shape()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let shape = match shape {
        Shape::String => SyntaxShape::String,
        Shape::Int => SyntaxShape::Int,
        Shape::Number => SyntaxShape::Number,
        Shape::Boolean => SyntaxShape::Boolean,
        Shape::Any => SyntaxShape::Any,
        Shape::None => SyntaxShape::Any,
    };

    Ok(PositionalArg {
        name: name.to_string(),
        desc: desc.to_string(),
        shape,
        var_id: None,
    })
}

fn deserialize_flag(reader: flag::Reader) -> Result<Flag, PluginError> {
    let long = reader
        .get_long()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let desc = reader
        .get_desc()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let required = reader.get_required();

    let short = reader
        .get_short()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let short = match short.which() {
        Err(capnp::NotInSchema(_)) => None,
        Ok(option::None(())) => None,
        Ok(option::Some(reader)) => {
            let reader = reader.map_err(|e| PluginError::EncodingError(e.to_string()))?;
            reader.chars().next()
        }
    };

    let arg = reader
        .get_arg()
        .map_err(|e| PluginError::EncodingError(e.to_string()))?;

    let arg = match arg {
        Shape::None => None,
        Shape::Any => Some(SyntaxShape::Any),
        Shape::String => Some(SyntaxShape::String),
        Shape::Int => Some(SyntaxShape::Int),
        Shape::Number => Some(SyntaxShape::Number),
        Shape::Boolean => Some(SyntaxShape::Boolean),
    };

    Ok(Flag {
        long: long.to_string(),
        short,
        arg,
        required,
        desc: desc.to_string(),
        var_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use capnp::serialize_packed;
    use nu_protocol::{Signature, SyntaxShape};

    pub fn write_buffer(
        signature: &Signature,
        writer: &mut impl std::io::Write,
    ) -> Result<(), PluginError> {
        let mut message = ::capnp::message::Builder::new_default();

        let builder = message.init_root::<signature::Builder>();

        serialize_signature(signature, builder);

        serialize_packed::write_message(writer, &message)
            .map_err(|e| PluginError::EncodingError(e.to_string()))
    }

    pub fn read_buffer(reader: &mut impl std::io::BufRead) -> Result<Signature, PluginError> {
        let message_reader =
            serialize_packed::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

        let reader = message_reader
            .get_root::<signature::Reader>()
            .map_err(|e| PluginError::DecodingError(e.to_string()))?;

        deserialize_signature(reader)
    }

    #[test]
    fn value_round_trip() {
        let signature = Signature::build("nu-plugin")
            .required("first", SyntaxShape::String, "first required")
            .required("second", SyntaxShape::Int, "second required")
            .required_named("first_named", SyntaxShape::String, "first named", Some('f'))
            .required_named(
                "second_named",
                SyntaxShape::String,
                "second named",
                Some('s'),
            )
            .rest("remaining", SyntaxShape::Int, "remaining");

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&signature, &mut buffer).expect("unable to serialize message");
        let returned_signature =
            read_buffer(&mut buffer.as_slice()).expect("unable to deserialize message");

        assert_eq!(signature.name, returned_signature.name);
        assert_eq!(signature.usage, returned_signature.usage);
        assert_eq!(signature.extra_usage, returned_signature.extra_usage);
        assert_eq!(signature.is_filter, returned_signature.is_filter);

        signature
            .required_positional
            .iter()
            .zip(returned_signature.required_positional.iter())
            .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

        signature
            .optional_positional
            .iter()
            .zip(returned_signature.optional_positional.iter())
            .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

        signature
            .named
            .iter()
            .zip(returned_signature.named.iter())
            .for_each(|(lhs, rhs)| assert_eq!(lhs, rhs));

        assert_eq!(
            signature.rest_positional,
            returned_signature.rest_positional,
        );
    }
}
