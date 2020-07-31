use crate::commands::du::{DirBuilder, DirInfo};
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

pub(crate) fn get_file_type(md: &std::fs::Metadata) -> &str {
    let ft = md.file_type();
    let mut file_type = "Unknown";
    if ft.is_dir() {
        file_type = "Dir";
    } else if ft.is_file() {
        file_type = "File";
    } else if ft.is_symlink() {
        file_type = "Symlink";
    } else {
        #[cfg(unix)]
        {
            if ft.is_block_device() {
                file_type = "Block device";
            } else if ft.is_char_device() {
                file_type = "Char device";
            } else if ft.is_fifo() {
                file_type = "Pipe";
            } else if ft.is_socket() {
                file_type = "Socket";
            }
        }
    }
    file_type
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dir_entry_dict(
    filename: &std::path::Path,
    metadata: Option<&std::fs::Metadata>,
    tag: impl Into<Tag>,
    long: bool,
    short_name: bool,
    with_symlink_targets: bool,
    du: bool,
    ctrl_c: Arc<AtomicBool>,
) -> Result<Value, ShellError> {
    let tag = tag.into();
    let mut dict = TaggedDictBuilder::new(&tag);
    // Insert all columns first to maintain proper table alignment if we can't find (or are not allowed to view) any information
    if long {
        #[cfg(windows)]
        {
            for column in [
                "name", "type", "target", "readonly", "size", "created", "accessed", "modified",
            ]
            .iter()
            {
                dict.insert_untagged(*column, UntaggedValue::nothing());
            }
        }

        #[cfg(unix)]
        {
            for column in [
                "name", "type", "target", "readonly", "mode", "uid", "group", "size", "created",
                "accessed", "modified",
            ]
            .iter()
            {
                dict.insert_untagged(&(*column.to_owned()), UntaggedValue::nothing());
            }
        }
    } else {
        for column in ["name", "type", "target", "size", "modified"].iter() {
            if *column == "target" && !with_symlink_targets {
                continue;
            }
            dict.insert_untagged(*column, UntaggedValue::nothing());
        }
    }

    let name = if short_name {
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
        dict.insert_untagged("type", get_file_type(md));
    }

    if long || with_symlink_targets {
        if let Some(md) = metadata {
            if md.file_type().is_symlink() {
                let symlink_target_untagged_value: UntaggedValue;
                if let Ok(path_to_link) = filename.read_link() {
                    symlink_target_untagged_value =
                        UntaggedValue::string(path_to_link.to_string_lossy());
                } else {
                    symlink_target_untagged_value =
                        UntaggedValue::string("Could not obtain target file's path");
                }
                dict.insert_untagged("target", symlink_target_untagged_value);
            }
        }
    }

    if long {
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
        }
    }

    if let Some(md) = metadata {
        let mut size_untagged_value: UntaggedValue = UntaggedValue::nothing();

        if md.is_dir() {
            let dir_size: u64 = if du {
                let params = DirBuilder::new(
                    Tag {
                        anchor: None,
                        span: Span::new(0, 2),
                    },
                    None,
                    false,
                    None,
                    false,
                );

                DirInfo::new(filename, &params, None, ctrl_c).get_size()
            } else {
                md.len()
            };

            size_untagged_value = UntaggedValue::filesize(dir_size);
        } else if md.is_file() {
            size_untagged_value = UntaggedValue::filesize(md.len());
        } else if md.file_type().is_symlink() {
            if let Ok(symlink_md) = filename.symlink_metadata() {
                size_untagged_value = UntaggedValue::filesize(symlink_md.len() as u64);
            }
        }

        dict.insert_untagged("size", size_untagged_value);
    }

    if let Some(md) = metadata {
        if long {
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
    }

    Ok(dict.into_value())
}
