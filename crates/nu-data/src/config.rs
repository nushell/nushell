mod conf;
mod nuconfig;

pub mod tests;

pub use conf::Conf;
pub use nuconfig::NuConfig;

use indexmap::IndexMap;
use log::trace;
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{
    Dictionary, Primitive, ShellTypeName, TaggedDictBuilder, UnspannedPathMember, UntaggedValue,
    Value,
};
use nu_source::{SpannedItem, Tag, TaggedItem};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub fn convert_toml_value_to_nu_value(v: &toml::Value, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    match v {
        toml::Value::Boolean(b) => UntaggedValue::boolean(*b).into_value(tag),
        toml::Value::Integer(n) => UntaggedValue::int(*n).into_value(tag),
        toml::Value::Float(n) => UntaggedValue::decimal_from_float(*n, tag.span).into_value(tag),
        toml::Value::String(s) => {
            UntaggedValue::Primitive(Primitive::String(String::from(s))).into_value(tag)
        }
        toml::Value::Array(a) => UntaggedValue::Table(
            a.iter()
                .map(|x| convert_toml_value_to_nu_value(x, &tag))
                .collect(),
        )
        .into_value(tag),
        toml::Value::Datetime(dt) => {
            UntaggedValue::Primitive(Primitive::String(dt.to_string())).into_value(tag)
        }
        toml::Value::Table(t) => {
            let mut collected = TaggedDictBuilder::new(&tag);

            for (k, v) in t.iter() {
                collected.insert_value(k.clone(), convert_toml_value_to_nu_value(v, &tag));
            }

            collected.into_value()
        }
    }
}

fn collect_values(input: &[Value]) -> Result<Vec<toml::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(helper(value)?);
    }

    Ok(out)
}
// Helper method to recursively convert nu_protocol::Value -> toml::Value
// This shouldn't be called at the top-level
fn helper(v: &Value) -> Result<toml::Value, ShellError> {
    use bigdecimal::ToPrimitive;

    Ok(match &v.value {
        UntaggedValue::Primitive(Primitive::Boolean(b)) => toml::Value::Boolean(*b),
        UntaggedValue::Primitive(Primitive::Filesize(b)) => {
            if let Some(value) = b.to_i64() {
                toml::Value::Integer(value)
            } else {
                return Err(ShellError::labeled_error(
                    "Value too large to convert to toml value",
                    "value too large",
                    v.tag.span,
                ));
            }
        }
        UntaggedValue::Primitive(Primitive::Duration(i)) => toml::Value::String(i.to_string()),
        UntaggedValue::Primitive(Primitive::Date(d)) => toml::Value::String(d.to_string()),
        UntaggedValue::Primitive(Primitive::EndOfStream) => {
            toml::Value::String("<End of Stream>".to_string())
        }
        UntaggedValue::Primitive(Primitive::BeginningOfStream) => {
            toml::Value::String("<Beginning of Stream>".to_string())
        }
        UntaggedValue::Primitive(Primitive::Decimal(f)) => {
            toml::Value::Float(f.tagged(&v.tag).coerce_into("converting to TOML float")?)
        }
        UntaggedValue::Primitive(Primitive::Int(i)) => {
            toml::Value::Integer(i.tagged(&v.tag).coerce_into("converting to TOML integer")?)
        }
        UntaggedValue::Primitive(Primitive::Nothing) => {
            toml::Value::String("<Nothing>".to_string())
        }
        UntaggedValue::Primitive(Primitive::GlobPattern(s)) => toml::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::String(s)) => toml::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::FilePath(s)) => {
            toml::Value::String(s.display().to_string())
        }
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => toml::Value::Array(
            path.iter()
                .map(|x| match &x.unspanned {
                    UnspannedPathMember::String(string) => Ok(toml::Value::String(string.clone())),
                    UnspannedPathMember::Int(int) => Ok(toml::Value::Integer(
                        int.tagged(&v.tag)
                            .coerce_into("converting to TOML integer")?,
                    )),
                })
                .collect::<Result<Vec<toml::Value>, ShellError>>()?,
        ),
        UntaggedValue::Table(l) => toml::Value::Array(collect_values(l)?),
        UntaggedValue::Error(e) => return Err(e.clone()),
        UntaggedValue::Block(_) => toml::Value::String("<Block>".to_string()),
        UntaggedValue::Primitive(Primitive::Range(_)) => toml::Value::String("<Range>".to_string()),
        UntaggedValue::Primitive(Primitive::Binary(b)) => {
            toml::Value::Array(b.iter().map(|x| toml::Value::Integer(*x as i64)).collect())
        }
        UntaggedValue::Row(o) => {
            let mut m = toml::map::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), helper(v)?);
            }
            toml::Value::Table(m)
        }
    })
}

/// Converts a nu_protocol::Value into a toml::Value
/// Will return a Shell Error, if the Nu Value is not a valid top-level TOML Value
pub fn value_to_toml_value(v: &Value) -> Result<toml::Value, ShellError> {
    match &v.value {
        UntaggedValue::Row(o) => {
            let mut m = toml::map::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), helper(v)?);
            }
            Ok(toml::Value::Table(m))
        }
        UntaggedValue::Primitive(Primitive::String(s)) => {
            // Attempt to de-serialize the String
            toml::de::from_str(s).map_err(|_| {
                ShellError::labeled_error(
                    format!("{:?} unable to de-serialize string to TOML", s),
                    "invalid TOML",
                    v.tag(),
                )
            })
        }
        _ => Err(ShellError::labeled_error(
            format!("{:?} is not a valid top-level TOML", v.value),
            "invalid TOML",
            v.tag(),
        )),
    }
}

#[cfg(feature = "directories")]
pub fn config_path() -> Result<PathBuf, ShellError> {
    use directories_next::ProjectDirs;

    let dir = ProjectDirs::from("org", "nushell", "nu")
        .ok_or_else(|| ShellError::untagged_runtime_error("Couldn't find project directory"))?;
    let path = ProjectDirs::config_dir(&dir).to_owned();
    std::fs::create_dir_all(&path).map_err(|err| {
        ShellError::untagged_runtime_error(&format!("Couldn't create {} path:\n{}", "config", err))
    })?;

    Ok(path)
}

#[cfg(not(feature = "directories"))]
pub fn config_path() -> Result<PathBuf, ShellError> {
    // FIXME: unsure if this should be error or a simple default

    Ok(std::path::PathBuf::from("/"))
}

pub fn default_path() -> Result<PathBuf, ShellError> {
    default_path_for(&None)
}

pub fn default_path_for(file: &Option<PathBuf>) -> Result<PathBuf, ShellError> {
    let mut filename = config_path()?;
    let file: &Path = file
        .as_ref()
        .map(AsRef::as_ref)
        .unwrap_or_else(|| "config.toml".as_ref());
    filename.push(file);

    Ok(filename)
}

#[cfg(feature = "directories")]
pub fn user_data() -> Result<PathBuf, ShellError> {
    use directories_next::ProjectDirs;

    let dir = ProjectDirs::from("org", "nushell", "nu")
        .ok_or_else(|| ShellError::untagged_runtime_error("Couldn't find project directory"))?;
    let path = ProjectDirs::data_local_dir(&dir).to_owned();
    std::fs::create_dir_all(&path).map_err(|err| {
        ShellError::untagged_runtime_error(&format!(
            "Couldn't create {} path:\n{}",
            "user data", err
        ))
    })?;

    Ok(path)
}

#[cfg(not(feature = "directories"))]
pub fn user_data() -> Result<PathBuf, ShellError> {
    // FIXME: unsure if this should be error or a simple default

    Ok(std::path::PathBuf::from("/"))
}

#[derive(Debug, Clone)]
pub enum Status {
    LastModified(std::time::SystemTime),
    Unavailable,
}

impl Default for Status {
    fn default() -> Self {
        Status::Unavailable
    }
}

pub fn last_modified(at: &Option<PathBuf>) -> Result<Status, Box<dyn std::error::Error>> {
    let filename = default_path()?;

    let filename = match at {
        None => filename,
        Some(ref file) => file.clone(),
    };

    if let Ok(time) = filename.metadata()?.modified() {
        return Ok(Status::LastModified(time));
    }

    Ok(Status::Unavailable)
}

pub fn read(
    tag: impl Into<Tag>,
    at: &Option<PathBuf>,
) -> Result<IndexMap<String, Value>, ShellError> {
    let filename = default_path()?;

    let filename = match at {
        None => filename,
        Some(ref file) => file.clone(),
    };

    if !filename.exists() && touch(&filename).is_err() {
        // If we can't create configs, let's just return an empty indexmap instead as we may be in
        // a readonly environment
        return Ok(IndexMap::new());
    }

    trace!("config file = {}", filename.display());

    let tag = tag.into();
    let contents = fs::read_to_string(filename)
        .map(|v| v.tagged(&tag))
        .map_err(|err| {
            ShellError::labeled_error(
                &format!("Couldn't read config file:\n{}", err),
                "file name",
                &tag,
            )
        })?;

    let parsed: toml::Value = toml::from_str(&contents).map_err(|err| {
        ShellError::labeled_error(
            &format!("Couldn't parse config file:\n{}", err),
            "file name",
            &tag,
        )
    })?;

    let value = convert_toml_value_to_nu_value(&parsed, tag);
    let tag = value.tag();
    match value.value {
        UntaggedValue::Row(Dictionary { entries }) => Ok(entries),
        other => Err(ShellError::type_error(
            "Dictionary",
            other.type_name().spanned(tag.span),
        )),
    }
}

pub fn config(tag: impl Into<Tag>) -> Result<IndexMap<String, Value>, ShellError> {
    read(tag, &None)
}

pub fn write(config: &IndexMap<String, Value>, at: &Option<PathBuf>) -> Result<(), ShellError> {
    let filename = &mut default_path()?;
    let filename = match at {
        None => filename,
        Some(file) => {
            filename.pop();
            filename.push(file);
            filename
        }
    };

    let contents = value_to_toml_value(
        &UntaggedValue::Row(Dictionary::new(config.clone())).into_untagged_value(),
    )?;

    let contents = toml::to_string(&contents)?;

    fs::write(&filename, &contents)?;

    Ok(())
}

// A simple implementation of `% touch path` (ignores existing files)
fn touch(path: &Path) -> io::Result<()> {
    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
