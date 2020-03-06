use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;

pub fn nu(tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();

    let mut nu_dict = TaggedDictBuilder::new(&tag);

    let mut dict = TaggedDictBuilder::new(&tag);
    for v in std::env::vars() {
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

    Ok(nu_dict.into_value())
}
