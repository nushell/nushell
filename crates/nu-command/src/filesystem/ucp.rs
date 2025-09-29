#[allow(deprecated)]
use nu_engine::{command_prelude::*, current_dir};
use nu_protocol::{
    NuGlob,
    shell_error::{self, io::IoError},
};
use std::path::PathBuf;
use uu_cp::{BackupMode, CopyMode, CpError, UpdateMode};
use uucore::{localized_help_template, translate};

// TODO: related to uucore::error::set_exit_code(EXIT_ERR)
// const EXIT_ERR: i32 = 1;

#[cfg(not(target_os = "windows"))]
const PATH_SEPARATOR: &str = "/";
#[cfg(target_os = "windows")]
const PATH_SEPARATOR: &str = "\\";

#[derive(Clone)]
pub struct UCp;

impl Command for UCp {
    fn name(&self) -> &str {
        "cp"
    }

    fn description(&self) -> &str {
        "Copy files using uutils/coreutils cp."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["copy", "file", "files", "coreutils"]
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch("recursive", "copy directories recursively", Some('r'))
            .switch("verbose", "explicitly state what is being done", Some('v'))
            .switch(
                "force",
                "if an existing destination file cannot be opened, remove it and try
                    again (this option is ignored when the -n option is also used).
                    currently not implemented for windows",
                Some('f'),
            )
            .switch("interactive", "ask before overwriting files", Some('i'))
            .switch(
                "update",
                "copy only when the SOURCE file is newer than the destination file or when the destination file is missing",
                Some('u')
            )
            .switch("progress", "display a progress bar", Some('p'))
            .switch("no-clobber", "do not overwrite an existing file", Some('n'))
            .named(
                "preserve",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "preserve only the specified attributes (empty list means no attributes preserved)
                    if not specified only mode is preserved
                    possible values: mode, ownership (unix only), timestamps, context, link, links, xattr",
                None
            )
            .switch("debug", "explain how a file is copied. Implies -v", None)
            .rest("paths", SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::String]), "Copy SRC file/s to DEST.")
            .allow_variants_without_examples(true)
            .category(Category::FileSystem)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Copy myfile to dir_b",
                example: "cp myfile dir_b",
                result: None,
            },
            Example {
                description: "Recursively copy dir_a to dir_b",
                example: "cp -r dir_a dir_b",
                result: None,
            },
            Example {
                description: "Recursively copy dir_a to dir_b, and print the feedbacks",
                example: "cp -r -v dir_a dir_b",
                result: None,
            },
            Example {
                description: "Move many files into a directory",
                example: "cp *.txt dir_a",
                result: None,
            },
            Example {
                description: "Copy only if source file is newer than target file",
                example: "cp -u myfile newfile",
                result: None,
            },
            Example {
                description: "Copy file preserving mode and timestamps attributes",
                example: "cp --preserve [ mode timestamps ] myfile newfile",
                result: None,
            },
            Example {
                description: "Copy file erasing all attributes",
                example: "cp --preserve [] myfile newfile",
                result: None,
            },
            Example {
                description: "Copy file to a directory three levels above its current location",
                example: "cp myfile ....",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // setup the uutils error translation
        let _ = localized_help_template("cp");

        let interactive = call.has_flag(engine_state, stack, "interactive")?;
        let (update, copy_mode) = if call.has_flag(engine_state, stack, "update")? {
            (UpdateMode::IfOlder, CopyMode::Update)
        } else {
            (UpdateMode::All, CopyMode::Copy)
        };

        let force = call.has_flag(engine_state, stack, "force")?;
        let no_clobber = call.has_flag(engine_state, stack, "no-clobber")?;
        let progress = call.has_flag(engine_state, stack, "progress")?;
        let recursive = call.has_flag(engine_state, stack, "recursive")?;
        let verbose = call.has_flag(engine_state, stack, "verbose")?;
        let preserve: Option<Value> = call.get_flag(engine_state, stack, "preserve")?;

        let debug = call.has_flag(engine_state, stack, "debug")?;
        let overwrite = if no_clobber {
            uu_cp::OverwriteMode::NoClobber
        } else if interactive {
            if force {
                uu_cp::OverwriteMode::Interactive(uu_cp::ClobberMode::Force)
            } else {
                uu_cp::OverwriteMode::Interactive(uu_cp::ClobberMode::Standard)
            }
        } else if force {
            uu_cp::OverwriteMode::Clobber(uu_cp::ClobberMode::Force)
        } else {
            uu_cp::OverwriteMode::Clobber(uu_cp::ClobberMode::Standard)
        };
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
        let reflink_mode = uu_cp::ReflinkMode::Auto;
        #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
        let reflink_mode = uu_cp::ReflinkMode::Never;
        let mut paths = call.rest::<Spanned<NuGlob>>(engine_state, stack, 0)?;
        if paths.is_empty() {
            return Err(ShellError::GenericError {
                error: "Missing file operand".into(),
                msg: "Missing file operand".into(),
                span: Some(call.head),
                help: Some("Please provide source and destination paths".into()),
                inner: vec![],
            });
        }

        if paths.len() == 1 {
            return Err(ShellError::GenericError {
                error: "Missing destination path".into(),
                msg: format!(
                    "Missing destination path operand after {}",
                    paths[0].item.as_ref()
                ),
                span: Some(paths[0].span),
                help: None,
                inner: vec![],
            });
        }
        let target = paths.pop().expect("Should not be reached?");
        let target_path = PathBuf::from(&nu_utils::strip_ansi_string_unlikely(
            target.item.to_string(),
        ));
        #[allow(deprecated)]
        let cwd = current_dir(engine_state, stack)?;
        let target_path = nu_path::expand_path_with(target_path, &cwd, target.item.is_expand());
        if target.item.as_ref().ends_with(PATH_SEPARATOR) && !target_path.is_dir() {
            return Err(ShellError::GenericError {
                error: "is not a directory".into(),
                msg: "is not a directory".into(),
                span: Some(target.span),
                help: None,
                inner: vec![],
            });
        };

        // paths now contains the sources

        let mut sources: Vec<(Vec<PathBuf>, bool)> = Vec::new();

        for mut p in paths {
            p.item = p.item.strip_ansi_string_unlikely();
            let exp_files: Vec<Result<PathBuf, ShellError>> =
                nu_engine::glob_from(&p, &cwd, call.head, None, engine_state.signals().clone())
                    .map(|f| f.1)?
                    .collect();
            if exp_files.is_empty() {
                return Err(ShellError::Io(IoError::new(
                    shell_error::io::ErrorKind::FileNotFound,
                    p.span,
                    PathBuf::from(p.item.to_string()),
                )));
            };
            let mut app_vals: Vec<PathBuf> = Vec::new();
            for v in exp_files {
                match v {
                    Ok(path) => {
                        if !recursive && path.is_dir() {
                            return Err(ShellError::GenericError {
                                error: "could_not_copy_directory".into(),
                                msg: "resolves to a directory (not copied)".into(),
                                span: Some(p.span),
                                help: Some(
                                    "Directories must be copied using \"--recursive\"".into(),
                                ),
                                inner: vec![],
                            });
                        };
                        app_vals.push(path)
                    }
                    Err(e) => return Err(e),
                }
            }
            sources.push((app_vals, p.item.is_expand()));
        }

        // Make sure to send absolute paths to avoid uu_cp looking for cwd in std::env which is not
        // supported in Nushell
        for (sources, need_expand_tilde) in sources.iter_mut() {
            for src in sources.iter_mut() {
                if !src.is_absolute() {
                    *src = nu_path::expand_path_with(&*src, &cwd, *need_expand_tilde);
                }
            }
        }
        let sources: Vec<PathBuf> = sources.into_iter().flat_map(|x| x.0).collect();

        let attributes = make_attributes(preserve)?;

        let options = uu_cp::Options {
            overwrite,
            reflink_mode,
            recursive,
            debug,
            attributes,
            verbose: verbose || debug,
            dereference: !recursive,
            progress_bar: progress,
            attributes_only: false,
            backup: BackupMode::None,
            copy_contents: false,
            cli_dereference: false,
            copy_mode,
            no_target_dir: false,
            one_file_system: false,
            parents: false,
            sparse_mode: uu_cp::SparseMode::Auto,
            strip_trailing_slashes: false,
            backup_suffix: String::from("~"),
            target_dir: None,
            update,
            set_selinux_context: false,
            context: None,
        };

        if let Err(error) = uu_cp::copy(&sources, &target_path, &options) {
            match error {
                // code should still be EXIT_ERR as does GNU cp
                CpError::NotAllFilesCopied => {}
                _ => {
                    eprintln!("here");
                    return Err(ShellError::GenericError {
                        error: format!("{error}"),
                        msg: translate!(&error.to_string()),
                        span: None,
                        help: None,
                        inner: vec![],
                    });
                }
            };
            // TODO: What should we do in place of set_exit_code?
            // uucore::error::set_exit_code(EXIT_ERR);
        }
        Ok(PipelineData::empty())
    }
}

const ATTR_UNSET: uu_cp::Preserve = uu_cp::Preserve::No { explicit: true };
const ATTR_SET: uu_cp::Preserve = uu_cp::Preserve::Yes { required: true };

fn make_attributes(preserve: Option<Value>) -> Result<uu_cp::Attributes, ShellError> {
    if let Some(preserve) = preserve {
        let mut attributes = uu_cp::Attributes {
            #[cfg(any(
                target_os = "linux",
                target_os = "freebsd",
                target_os = "android",
                target_os = "macos",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            ownership: ATTR_UNSET,
            mode: ATTR_UNSET,
            timestamps: ATTR_UNSET,
            context: ATTR_UNSET,
            links: ATTR_UNSET,
            xattr: ATTR_UNSET,
        };
        parse_and_set_attributes_list(&preserve, &mut attributes)?;

        Ok(attributes)
    } else {
        // By default preseerve only mode
        Ok(uu_cp::Attributes {
            mode: ATTR_SET,
            #[cfg(any(
                target_os = "linux",
                target_os = "freebsd",
                target_os = "android",
                target_os = "macos",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            ownership: ATTR_UNSET,
            timestamps: ATTR_UNSET,
            context: ATTR_UNSET,
            links: ATTR_UNSET,
            xattr: ATTR_UNSET,
        })
    }
}

fn parse_and_set_attributes_list(
    list: &Value,
    attribute: &mut uu_cp::Attributes,
) -> Result<(), ShellError> {
    match list {
        Value::List { vals, .. } => {
            for val in vals {
                parse_and_set_attribute(val, attribute)?;
            }
            Ok(())
        }
        _ => Err(ShellError::IncompatibleParametersSingle {
            msg: "--preserve flag expects a list of strings".into(),
            span: list.span(),
        }),
    }
}

fn parse_and_set_attribute(
    value: &Value,
    attribute: &mut uu_cp::Attributes,
) -> Result<(), ShellError> {
    match value {
        Value::String { val, .. } => {
            let attribute = match val.as_str() {
                "mode" => &mut attribute.mode,
                #[cfg(any(
                    target_os = "linux",
                    target_os = "freebsd",
                    target_os = "android",
                    target_os = "macos",
                    target_os = "netbsd",
                    target_os = "openbsd"
                ))]
                "ownership" => &mut attribute.ownership,
                "timestamps" => &mut attribute.timestamps,
                "context" => &mut attribute.context,
                "link" | "links" => &mut attribute.links,
                "xattr" => &mut attribute.xattr,
                _ => {
                    return Err(ShellError::IncompatibleParametersSingle {
                        msg: format!("--preserve flag got an unexpected attribute \"{val}\""),
                        span: value.span(),
                    });
                }
            };
            *attribute = ATTR_SET;
            Ok(())
        }
        _ => Err(ShellError::IncompatibleParametersSingle {
            msg: "--preserve flag expects a list of strings".into(),
            span: value.span(),
        }),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(UCp {})
    }
}
