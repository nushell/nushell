use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};

pub(crate) fn dir_entry_dict(
    filename: &std::path::Path,
    metadata: &std::fs::Metadata,
    tag: impl Into<Tag>,
    full: bool,
    name_only: bool,
    with_symlink_targets: bool,
) -> Result<Value, ShellError> {
    let tag = tag.into();
    let mut dict = TaggedDictBuilder::new(&tag);

    let name = if name_only {
        filename.file_name().and_then(|s| s.to_str())
    } else {
        filename.to_str()
    }
    .ok_or_else(|| {
        ShellError::labeled_error(
            format!("Invalid file name: {:}", filename.to_string_lossy()),
            "invalid file name",
            tag,
        )
    })?;

    dict.insert_untagged("name", UntaggedValue::string(name));

    if metadata.is_dir() {
        dict.insert_untagged("type", UntaggedValue::string("Dir"));
    } else if metadata.is_file() {
        dict.insert_untagged("type", UntaggedValue::string("File"));
    } else {
        dict.insert_untagged("type", UntaggedValue::string("Symlink"));
    };

    if full || with_symlink_targets {
        if metadata.is_dir() || metadata.is_file() {
            dict.insert_untagged("target", UntaggedValue::bytes(0u64));
        } else if let Ok(path_to_link) = filename.read_link() {
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

    if metadata.is_file() {
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
