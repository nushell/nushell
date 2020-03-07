use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};

pub(crate) fn dir_entry_dict(
    filename: &std::path::Path,
    metadata: Option<&std::fs::Metadata>,
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

    if let Some(md) = metadata {
        let ft = md.file_type();
        if ft.is_dir() {
            dict.insert_untagged("type", UntaggedValue::string("Dir"));
        } else if ft.is_file() {
            dict.insert_untagged("type", UntaggedValue::string("File"));
        } else if ft.is_symlink() {
            dict.insert_untagged("type", UntaggedValue::string("Symlink"));
        } else {
            dict.insert_untagged("type", UntaggedValue::string("Unknown"));
        }
    } else {
        dict.insert_untagged("type", UntaggedValue::nothing());
    }

    if full || with_symlink_targets {
        if let Some(md) = metadata {
            let ft = md.file_type();
            if ft.is_symlink() {
                if let Ok(path_to_link) = filename.read_link() {
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
            else {
                dict.insert_untagged("target", UntaggedValue::nothing());
            }
        } else {
            dict.insert_untagged("target", UntaggedValue::nothing());
        }
    }

    if full {
        if let Some(md) = metadata {
            dict.insert_untagged(
                "readonly",
                UntaggedValue::boolean(md.permissions().readonly()),
            );

            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                use std::os::unix::fs::PermissionsExt;
                let mode = md.permissions().mode();
                dict.insert_untagged(
                    "mode",
                    UntaggedValue::string(umask::Mode::from(mode).to_string()),
                );

                if let Some(user) = users::get_user_by_uid(md.uid()) {
                    dict.insert_untagged(
                        "uid",
                        UntaggedValue::string(user.name().to_string_lossy()),
                    );
                }

                if let Some(group) = users::get_group_by_gid(md.gid()) {
                    dict.insert_untagged(
                        "group",
                        UntaggedValue::string(group.name().to_string_lossy()),
                    );
                }
            }
        } else {
            dict.insert_untagged("readonly", UntaggedValue::nothing());

            #[cfg(unix)]
            {
                dict.insert_untagged("mode", UntaggedValue::nothing());
            }
        }
    }

    if let Some(md) = metadata {
        if md.is_file() {
            dict.insert_untagged("size", UntaggedValue::bytes(md.len() as u64));
        } else {
            dict.insert_untagged("size", UntaggedValue::nothing());
        }
    } else {
        dict.insert_untagged("size", UntaggedValue::nothing());
    }

    if let Some(md) = metadata {
        if full {
            if let Ok(c) = md.created() {
                dict.insert_untagged("created", UntaggedValue::system_date(c));
            }

            if let Ok(a) = md.accessed() {
                dict.insert_untagged("accessed", UntaggedValue::system_date(a));
            }
        }

        if let Ok(m) = md.modified() {
            dict.insert_untagged("modified", UntaggedValue::system_date(m));
        }
    } else {
        if full {
            dict.insert_untagged("created", UntaggedValue::nothing());
            dict.insert_untagged("accessed", UntaggedValue::nothing());
        }

        dict.insert_untagged("modified", UntaggedValue::nothing());
    }

    Ok(dict.into_value())
}
