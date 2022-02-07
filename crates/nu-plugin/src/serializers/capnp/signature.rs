use crate::plugin_capnp::{argument, flag, signature, Category as PluginCategory, Shape};
use nu_protocol::{Category, Flag, PositionalArg, ShellError, Signature, SyntaxShape};

pub(crate) fn serialize_signature(signature: &Signature, mut builder: signature::Builder) {
    builder.set_name(signature.name.as_str());
    builder.set_usage(signature.usage.as_str());
    builder.set_extra_usage(signature.extra_usage.as_str());
    builder.set_is_filter(signature.is_filter);

    match signature.category {
        Category::Default => builder.set_category(PluginCategory::Default),
        Category::Conversions => builder.set_category(PluginCategory::Conversions),
        Category::Core => builder.set_category(PluginCategory::Core),
        Category::Date => builder.set_category(PluginCategory::Date),
        Category::Env => builder.set_category(PluginCategory::Env),
        Category::Experimental => builder.set_category(PluginCategory::Experimental),
        Category::FileSystem => builder.set_category(PluginCategory::Filesystem),
        Category::Filters => builder.set_category(PluginCategory::Filters),
        Category::Formats => builder.set_category(PluginCategory::Formats),
        Category::Math => builder.set_category(PluginCategory::Math),
        Category::Network => builder.set_category(PluginCategory::Network),
        Category::Random => builder.set_category(PluginCategory::Random),
        Category::Platform => builder.set_category(PluginCategory::Platform),
        Category::Shells => builder.set_category(PluginCategory::Shells),
        Category::Strings => builder.set_category(PluginCategory::Strings),
        Category::System => builder.set_category(PluginCategory::System),
        Category::Viewers => builder.set_category(PluginCategory::Viewers),
        Category::Hash => builder.set_category(PluginCategory::Hash),
        Category::Generators => builder.set_category(PluginCategory::Generators),
        _ => builder.set_category(PluginCategory::Default),
    }

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
    if let Some(arg) = &signature.rest_positional {
        let rest_argument = builder.reborrow().init_rest();
        serialize_argument(arg, rest_argument)
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

    if let Some(val) = arg.short {
        let mut inner_builder = builder.reborrow().init_short(1);
        inner_builder.push_str(format!("{}", val).as_str());
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

pub(crate) fn deserialize_signature(reader: signature::Reader) -> Result<Signature, ShellError> {
    let name = reader
        .get_name()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;
    let usage = reader
        .get_usage()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;
    let extra_usage = reader
        .get_extra_usage()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;
    let is_filter = reader.get_is_filter();

    let category = match reader
        .get_category()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?
    {
        PluginCategory::Default => Category::Default,
        PluginCategory::Conversions => Category::Conversions,
        PluginCategory::Core => Category::Core,
        PluginCategory::Date => Category::Date,
        PluginCategory::Env => Category::Env,
        PluginCategory::Experimental => Category::Experimental,
        PluginCategory::Filesystem => Category::FileSystem,
        PluginCategory::Filters => Category::Filters,
        PluginCategory::Formats => Category::Formats,
        PluginCategory::Math => Category::Math,
        PluginCategory::Strings => Category::Strings,
        PluginCategory::System => Category::System,
        PluginCategory::Viewers => Category::Viewers,
        PluginCategory::Network => Category::Network,
        PluginCategory::Random => Category::Random,
        PluginCategory::Platform => Category::Platform,
        PluginCategory::Shells => Category::Shells,
        PluginCategory::Hash => Category::Hash,
        PluginCategory::Generators => Category::Generators,
    };

    // Deserializing required arguments
    let required_list = reader
        .get_required_positional()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let required_positional = required_list
        .iter()
        .map(deserialize_argument)
        .collect::<Result<Vec<PositionalArg>, ShellError>>()?;

    // Deserializing optional arguments
    let optional_list = reader
        .get_optional_positional()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let optional_positional = optional_list
        .iter()
        .map(deserialize_argument)
        .collect::<Result<Vec<PositionalArg>, ShellError>>()?;

    // Deserializing rest arguments
    let rest_positional = if reader.has_rest() {
        let argument_reader = reader
            .get_rest()
            .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

        Some(deserialize_argument(argument_reader)?)
    } else {
        None
    };

    // Deserializing named arguments
    let named_list = reader
        .get_named()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let named = named_list
        .iter()
        .map(deserialize_flag)
        .collect::<Result<Vec<Flag>, ShellError>>()?;

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
        category,
    })
}

fn deserialize_argument(reader: argument::Reader) -> Result<PositionalArg, ShellError> {
    let name = reader
        .get_name()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let desc = reader
        .get_desc()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let shape = reader
        .get_shape()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

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

fn deserialize_flag(reader: flag::Reader) -> Result<Flag, ShellError> {
    let long = reader
        .get_long()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let desc = reader
        .get_desc()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

    let required = reader.get_required();

    let short = if reader.has_short() {
        let short_reader = reader
            .get_short()
            .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

        short_reader.chars().next()
    } else {
        None
    };

    let arg = reader
        .get_arg()
        .map_err(|e| ShellError::PluginFailedToDecode(e.to_string()))?;

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
    use capnp::serialize;
    use nu_protocol::{Category, Signature, SyntaxShape};

    pub fn write_buffer(
        signature: &Signature,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ShellError> {
        let mut message = ::capnp::message::Builder::new_default();

        let builder = message.init_root::<signature::Builder>();

        serialize_signature(signature, builder);

        serialize::write_message(writer, &message)
            .map_err(|e| ShellError::PluginFailedToEncode(e.to_string()))
    }

    pub fn read_buffer(reader: &mut impl std::io::BufRead) -> Result<Signature, ShellError> {
        let message_reader =
            serialize::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

        let reader = message_reader
            .get_root::<signature::Reader>()
            .map_err(|e| ShellError::PluginFailedToEncode(e.to_string()))?;

        deserialize_signature(reader)
    }

    #[test]
    fn value_round_trip() {
        let signature = Signature::build("nu-plugin")
            .required("first", SyntaxShape::String, "first required")
            .required("second", SyntaxShape::Int, "second required")
            .required_named("first_named", SyntaxShape::String, "first named", Some('f'))
            .required_named("second_named", SyntaxShape::Int, "first named", Some('s'))
            .required_named("name", SyntaxShape::String, "first named", Some('n'))
            .required_named("string", SyntaxShape::String, "second named", Some('x'))
            .switch("switch", "some switch", None)
            .rest("remaining", SyntaxShape::Int, "remaining")
            .category(Category::Conversions);

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&signature, &mut buffer).expect("unable to serialize message");
        let returned_signature =
            read_buffer(&mut buffer.as_slice()).expect("unable to deserialize message");

        assert_eq!(signature.name, returned_signature.name);
        assert_eq!(signature.usage, returned_signature.usage);
        assert_eq!(signature.extra_usage, returned_signature.extra_usage);
        assert_eq!(signature.is_filter, returned_signature.is_filter);
        assert_eq!(signature.category, returned_signature.category);

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

    #[test]
    fn value_round_trip_2() {
        let signature = Signature::build("test-1")
            .desc("Signature test 1 for plugin. Returns Value::Nothing")
            .required("a", SyntaxShape::Int, "required integer value")
            .required("b", SyntaxShape::String, "required string value")
            .optional("opt", SyntaxShape::Boolean, "Optional boolean")
            .switch("flag", "a flag for the signature", Some('f'))
            .named("named", SyntaxShape::String, "named string", Some('n'))
            .category(Category::Experimental);

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&signature, &mut buffer).expect("unable to serialize message");
        let returned_signature =
            read_buffer(&mut buffer.as_slice()).expect("unable to deserialize message");

        assert_eq!(signature.name, returned_signature.name);
        assert_eq!(signature.usage, returned_signature.usage);
        assert_eq!(signature.extra_usage, returned_signature.extra_usage);
        assert_eq!(signature.is_filter, returned_signature.is_filter);
        assert_eq!(signature.category, returned_signature.category);

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
