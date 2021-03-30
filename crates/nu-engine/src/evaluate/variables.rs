use crate::evaluate::scope::Scope;
use indexmap::IndexMap;
use nu_data::config::path::history_path;
use nu_errors::ShellError;
use nu_protocol::{Primitive, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::{Spanned, Tag};

pub fn nu(scope: &Scope, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let env = &scope.get_env_vars();
    let tag = tag.into();

    let mut nu_dict = TaggedDictBuilder::new(&tag);

    let mut dict = TaggedDictBuilder::new(&tag);
    for v in env.iter() {
        if v.0 != "PATH" && v.0 != "Path" {
            dict.insert_untagged(v.0, UntaggedValue::string(v.1));
        }
    }
    nu_dict.insert_value("env", dict.into_value());

    let config_file = match scope.get_var("config-path") {
        Some(Value {
            value: UntaggedValue::Primitive(Primitive::FilePath(path)),
            ..
        }) => Some(path),
        _ => None,
    };

    let config = nu_data::config::read(&tag, &config_file)?;
    nu_dict.insert_value("config", UntaggedValue::row(config).into_value(&tag));

    let mut table = vec![];
    for v in env.iter() {
        if v.0 == "PATH" || v.0 == "Path" {
            for path in std::env::split_paths(&v.1) {
                table.push(UntaggedValue::filepath(path).into_value(&tag));
            }
        }
    }
    nu_dict.insert_value("path", UntaggedValue::table(&table).into_value(&tag));

    let path = std::env::current_dir()?;
    nu_dict.insert_value("cwd", UntaggedValue::filepath(path).into_value(&tag));

    if let Some(home) = crate::filesystem::filesystem_shell::homedir_if_possible() {
        nu_dict.insert_value("home-dir", UntaggedValue::filepath(home).into_value(&tag));
    }

    let temp = std::env::temp_dir();
    nu_dict.insert_value("temp-dir", UntaggedValue::filepath(temp).into_value(&tag));

    let config = if let Some(path) = config_file {
        path
    } else {
        nu_data::config::default_path()?
    };

    nu_dict.insert_value(
        "config-path",
        UntaggedValue::filepath(config).into_value(&tag),
    );

    #[cfg(feature = "rustyline-support")]
    {
        let keybinding_path = nu_data::keybinding::keybinding_path()?;
        nu_dict.insert_value(
            "keybinding-path",
            UntaggedValue::filepath(keybinding_path).into_value(&tag),
        );
    }

    let config: Box<dyn nu_data::config::Conf> = Box::new(nu_data::config::NuConfig::new());
    let history = history_path(&config);
    nu_dict.insert_value(
        "history-path",
        UntaggedValue::filepath(history).into_value(&tag),
    );

    Ok(nu_dict.into_value())
}

pub fn scope(
    aliases: &IndexMap<String, Vec<Spanned<String>>>,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut scope_dict = TaggedDictBuilder::new(&tag);

    let mut dict = TaggedDictBuilder::new(&tag);
    for v in aliases.iter() {
        let values = v.1.clone();
        let mut vec = Vec::new();

        for k in values.iter() {
            vec.push(k.to_string());
        }

        let alias = vec.join(" ");
        dict.insert_untagged(v.0, UntaggedValue::string(alias));
    }

    scope_dict.insert_value("aliases", dict.into_value());
    Ok(scope_dict.into_value())
}
