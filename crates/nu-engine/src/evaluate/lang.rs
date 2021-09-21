use crate::Scope;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue, Value};
use nu_source::Tag;

pub struct Lang;

impl Lang {
    pub fn query_commands(scope: &Scope) -> Result<Vec<Value>, ShellError> {
        let tag = Tag::unknown();
        let full_commands = scope.get_commands_info();
        let mut cmd_vec = Vec::new();
        for (key, cmd) in full_commands {
            let mut indexmap = IndexMap::new();
            let mut sig = cmd.signature();
            // eprintln!("{}", get_signature(&sig));
            indexmap.insert(
                "name".to_string(),
                UntaggedValue::string(key).into_value(&tag),
            );
            indexmap.insert(
                "usage".to_string(),
                UntaggedValue::string(cmd.usage().to_string()).into_value(&tag),
            );
            // let sig_deser = serde_json::to_string(&sig).unwrap();
            // indexmap.insert(
            //     "signature".to_string(),
            //     UntaggedValue::string(sig_deser).into_value(&tag),
            // );
            let signature_table = get_signature(&mut sig, tag.clone());
            indexmap.insert(
                "signature".to_string(),
                UntaggedValue::Table(signature_table).into_value(&tag),
            );
            indexmap.insert(
                "is_filter".to_string(),
                UntaggedValue::boolean(sig.is_filter).into_value(&tag),
            );
            indexmap.insert(
                "is_builtin".to_string(),
                UntaggedValue::boolean(cmd.is_builtin()).into_value(&tag),
            );
            indexmap.insert(
                "is_sub".to_string(),
                UntaggedValue::boolean(cmd.is_sub()).into_value(&tag),
            );
            indexmap.insert(
                "is_plugin".to_string(),
                UntaggedValue::boolean(cmd.is_plugin()).into_value(&tag),
            );
            indexmap.insert(
                "is_custom".to_string(),
                UntaggedValue::boolean(cmd.is_custom()).into_value(&tag),
            );
            indexmap.insert(
                "is_private".to_string(),
                UntaggedValue::boolean(cmd.is_private()).into_value(&tag),
            );
            indexmap.insert(
                "is_binary".to_string(),
                UntaggedValue::boolean(cmd.is_binary()).into_value(&tag),
            );
            indexmap.insert(
                "extra_usage".to_string(),
                UntaggedValue::string(cmd.extra_usage().to_string()).into_value(&tag),
            );

            cmd_vec.push(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag));
        }

        Ok(cmd_vec)
    }
}

fn get_signature(sig: &mut Signature, tag: Tag) -> Vec<Value> {
    sig.remove_named("help");
    let p = &sig.positional;
    let r = &sig.rest_positional;
    let n = &sig.named;
    let name = &sig.name;
    let mut sig_vec: Vec<Value> = Vec::new();

    for item in p {
        let mut indexmap = IndexMap::new();

        let (parameter, syntax_shape) = item.0.get_type_description();
        let description = &item.1;
        // let output = format!(
        //     "Positional|{}|{}|{}|{}\n",
        //     name, parameter, syntax_shape, description
        // );
        // eprintln!("{}", output);

        indexmap.insert(
            "cmd_name".to_string(),
            UntaggedValue::string(name).into_value(&tag),
        );
        indexmap.insert(
            "parameter_name".to_string(),
            UntaggedValue::string(parameter).into_value(&tag),
        );
        indexmap.insert(
            "parameter_type".to_string(),
            UntaggedValue::string("positional".to_string()).into_value(&tag),
        );
        indexmap.insert(
            "syntax_shape".to_string(),
            UntaggedValue::string(syntax_shape).into_value(&tag),
        );
        indexmap.insert(
            "description".to_string(),
            UntaggedValue::string(description).into_value(&tag),
        );
        indexmap.insert(
            "flag_name".to_string(),
            UntaggedValue::string("".to_string()).into_value(&tag),
        );
        indexmap.insert(
            "flag_type".to_string(),
            UntaggedValue::string("".to_string()).into_value(&tag),
        );

        sig_vec.push(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag));
    }

    match r {
        Some((rest_name, shape, desc)) => {
            let mut indexmap = IndexMap::new();
            // let output = format!("Rest|{}|{}|{}\n", name, shape.syntax_shape_name(), desc);
            // eprintln!("{}", output);

            indexmap.insert(
                "cmd_name".to_string(),
                UntaggedValue::string(name).into_value(&tag),
            );
            indexmap.insert(
                "parameter_name".to_string(),
                UntaggedValue::string(rest_name.to_string()).into_value(&tag),
            );
            indexmap.insert(
                "parameter_type".to_string(),
                UntaggedValue::string("rest".to_string()).into_value(&tag),
            );
            indexmap.insert(
                "syntax_shape".to_string(),
                UntaggedValue::string(shape.syntax_shape_name()).into_value(&tag),
            );
            indexmap.insert(
                "description".to_string(),
                UntaggedValue::string(desc).into_value(&tag),
            );
            indexmap.insert(
                "flag_name".to_string(),
                UntaggedValue::string("".to_string()).into_value(&tag),
            );
            indexmap.insert(
                "flag_type".to_string(),
                UntaggedValue::string("".to_string()).into_value(&tag),
            );

            sig_vec.push(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag));
        }
        None => {}
    }

    for (parameter, (b, description)) in n {
        let mut indexmap = IndexMap::new();

        let (named_type, flag_name, shape) = b.get_type_description();
        // let output = format!(
        //     "Named|{}|{}|{}|{}|{}|{}\n",
        //     name, parameter, named_type, flag_name, shape, description
        // );
        // eprint!("{}", output);

        indexmap.insert(
            "cmd_name".to_string(),
            UntaggedValue::string(name).into_value(&tag),
        );
        indexmap.insert(
            "parameter_name".to_string(),
            UntaggedValue::string(parameter).into_value(&tag),
        );
        indexmap.insert(
            "parameter_type".to_string(),
            UntaggedValue::string("named".to_string()).into_value(&tag),
        );
        indexmap.insert(
            "syntax_shape".to_string(),
            UntaggedValue::string(shape).into_value(&tag),
        );
        indexmap.insert(
            "description".to_string(),
            UntaggedValue::string(description).into_value(&tag),
        );
        indexmap.insert(
            "flag_name".to_string(),
            UntaggedValue::string(flag_name).into_value(&tag),
        );
        indexmap.insert(
            "flag_type".to_string(),
            UntaggedValue::string(named_type).into_value(&tag),
        );
        sig_vec.push(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag));
    }

    sig_vec
}
