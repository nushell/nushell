use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use std::path::PathBuf;

pub(crate) fn dir_entry_dict(
    path: &PathBuf,
    tag: impl Into<Tag>,
    full: bool,
    with_symlink_targets: bool,
    short_name: bool,
) -> Result<Value, ShellError> {
    let tag = tag.into();
    let metadata = match std::fs::metadata(path) {
        Ok(m) => Ok(m),
        Err(e) => Err(ShellError::from(e)),
    }?;
    let file_type = metadata.file_type();

    let mut dict = TaggedDictBuilder::new(&tag);

    let name = if short_name {
        match path.file_name() {
            Some(n) => Ok(n.to_str().expect("This path was not properly encoded.")),
            None => Err(ShellError::labeled_error(
                format!("Invalid File name: {:}", path.to_string_lossy()),
                "Invalid File Name",
                tag,
            )),
        }
    } else {
        Ok(path.to_str().expect("This path was not properly encoded."))
    }?;

    dict.insert_untagged("name", UntaggedValue::string(name));

    if file_type.is_dir() {
        dict.insert_untagged("type", UntaggedValue::string("Dir"));
    } else if file_type.is_file() {
        dict.insert_untagged("type", UntaggedValue::string("File"));
    } else {
        dict.insert_untagged("type", UntaggedValue::string("Symlink"));
    };

    if full || with_symlink_targets {
        if metadata.is_dir() || metadata.is_file() {
            dict.insert_untagged("target", UntaggedValue::bytes(0u64));
        } else if let Ok(path_to_link) = path.read_link() {
            dict.insert_untagged(
                "target",
                UntaggedValue::string(path_to_link.to_string_lossy()),
            );
        } else {
            dict.insert_untagged(
                "target",
                UntaggedValue::string("Could not obtain target file's path"),
            );
        }
    }

    if full {
        dict.insert_untagged(
            "readonly",
            UntaggedValue::boolean(metadata.permissions().readonly()),
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            dict.insert_untagged(
                "mode",
                UntaggedValue::string(umask::Mode::from(mode).to_string()),
            );

            if let Some(user) = users::get_user_by_uid(metadata.uid()) {
                dict.insert_untagged("uid", UntaggedValue::string(user.name().to_string_lossy()));
            }

            if let Some(group) = users::get_group_by_gid(metadata.gid()) {
                dict.insert_untagged(
                    "group",
                    UntaggedValue::string(group.name().to_string_lossy()),
                );
            }
        }
    }

    if file_type.is_file() {
        dict.insert_untagged("size", UntaggedValue::bytes(metadata.len() as u64));
    } else {
        dict.insert_untagged("size", UntaggedValue::bytes(0u64));
    }

    if full {
        if let Ok(c) = metadata.created() {
            dict.insert_untagged("created", UntaggedValue::system_date(c));
        }

        if let Ok(a) = metadata.accessed() {
            dict.insert_untagged("accessed", UntaggedValue::system_date(a));
        }
    }

    if let Ok(m) = metadata.modified() {
        dict.insert_untagged("modified", UntaggedValue::system_date(m));
    }

    Ok(dict.into_value())
}
