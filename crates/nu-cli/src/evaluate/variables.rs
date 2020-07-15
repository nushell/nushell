use crate::cli::History;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;

pub fn nu(env: &IndexMap<String, String>, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut nu_dict = TaggedDictBuilder::new(&tag);

    let mut dict = TaggedDictBuilder::new(&tag);
    for v in env.iter() {
        if v.0 != "PATH" && v.0 != "Path" {
            dict.insert_untagged(v.0, UntaggedValue::string(v.1));
        }
    }
    nu_dict.insert_value("env", dict.into_value());

    let config = crate::data::config::read(&tag, &None)?;
    nu_dict.insert_value("config", UntaggedValue::row(config).into_value(&tag));

    let mut table = vec![];
    let path = std::env::var_os("PATH");
    if let Some(paths) = path {
        for path in std::env::split_paths(&paths) {
            table.push(UntaggedValue::path(path).into_value(&tag));
        }
    }
    nu_dict.insert_value("path", UntaggedValue::table(&table).into_value(&tag));

    let path = std::env::current_dir()?;
    nu_dict.insert_value("cwd", UntaggedValue::path(path).into_value(&tag));

    if let Some(home) = crate::shell::filesystem_shell::homedir_if_possible() {
        nu_dict.insert_value("home-dir", UntaggedValue::path(home).into_value(&tag));
    }

    let temp = std::env::temp_dir();
    nu_dict.insert_value("temp-dir", UntaggedValue::path(temp).into_value(&tag));

    let config = crate::data::config::default_path()?;
    nu_dict.insert_value("config-path", UntaggedValue::path(config).into_value(&tag));

    let keybinding_path = crate::keybinding::keybinding_path()?;
    nu_dict.insert_value(
        "keybinding-path",
        UntaggedValue::path(keybinding_path).into_value(&tag),
    );

    let history = History::path();
    nu_dict.insert_value(
        "history-path",
        UntaggedValue::path(history).into_value(&tag),
    );

    Ok(nu_dict.into_value())
}
