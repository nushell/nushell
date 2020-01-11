use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};

pub(crate) fn dir_entry_dict(
    filename: &std::path::Path,
    metadata: &std::fs::Metadata,
    tag: impl Into<Tag>,
    full: bool,
) -> Result<Value, ShellError> {
    let mut dict = TaggedDictBuilder::new(tag);
    dict.insert_untagged("name", UntaggedValue::string(filename.to_string_lossy()));

    if metadata.is_dir() {
        dict.insert_untagged("type", UntaggedValue::string("Dir"));
    } else if metadata.is_file() {
        dict.insert_untagged("type", UntaggedValue::string("File"));
    } else {
        dict.insert_untagged("type", UntaggedValue::string("Symlink"));
    };

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

    dict.insert_untagged("size", UntaggedValue::bytes(metadata.len() as u64));

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
