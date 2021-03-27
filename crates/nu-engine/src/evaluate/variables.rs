use crate::evaluate::scope::Scope;
use nu_data::config::NuConfig;
use nu_errors::ShellError;
use nu_protocol::{Primitive, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;

pub fn nu(scope: &Scope, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();

    let env = &scope.get_env_vars();

    let config = if let Some(Value {
        value: UntaggedValue::Primitive(Primitive::FilePath(path)),
        ..
    }) = scope.get_var("config-path")
    {
        NuConfig::with(Some(path).map(|p| p.into_os_string()))
    } else {
        NuConfig::new()
    };

    let mut nu_dict = TaggedDictBuilder::new(&tag);

    let mut dict = TaggedDictBuilder::new(&tag);
    for v in env.iter() {
        if v.0 != "PATH" && v.0 != "Path" {
            dict.insert_untagged(v.0, UntaggedValue::string(v.1));
        }
    }
    nu_dict.insert_value("env", dict.into_value());

    nu_dict.insert_value(
        "config",
        UntaggedValue::row(config.vars.clone()).into_value(&tag),
    );

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

    nu_dict.insert_value(
        "config-path",
        UntaggedValue::filepath(nu_data::config::path::source_file(&config)).into_value(&tag),
    );

    #[cfg(feature = "rustyline-support")]
    {
        let keybinding_path = nu_data::keybinding::keybinding_path()?;
        nu_dict.insert_value(
            "keybinding-path",
            UntaggedValue::filepath(keybinding_path).into_value(&tag),
        );
    }

    nu_dict.insert_value(
        "history-path",
        UntaggedValue::filepath(nu_data::config::path::history(&config)).into_value(&tag),
    );

    Ok(nu_dict.into_value())
}
