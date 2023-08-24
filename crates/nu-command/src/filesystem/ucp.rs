use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::{Argument, Expr};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
};

use std::path::PathBuf;
use uucore::backup_control::{self, BackupMode};
use uucore::update_control::{self, UpdateMode};

const EXIT_ERR: i32 = 1;
const GLOB_PARAMS: nu_glob::MatchOptions = nu_glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
    recursive_match_hidden_dir: true,
};

/// Returns tuple of (Source paths, Target)
pub fn parse_path_args(
    mut paths: Vec<PathBuf>,
    options: &uu_cp::Options,
) -> Result<(Vec<PathBuf>, PathBuf), ShellError> {
    if paths.is_empty() {
        // No files specified
        return Err(ShellError::GenericError(
            "Missing file operand".into(),
            "".into(),
            None,
            None,
            Vec::new(),
        ));
        // return Err("missing file operand".into());
    }

    // Return an error if the user requested to copy more than one
    // file source to a file target
    if options.no_target_dir && options.target_dir.is_none() && paths.len() > 2 {
        // FIX THIS ERROR PROPERLY
        return Err(ShellError::GenericError(
            "extra operand".into(),
            "".into(),
            None,
            None,
            Vec::new(),
        ));
        // return Err(format!("extra operand {:?}", paths[2]).into());
    }

    let target = match options.target_dir {
        Some(ref target) => {
            // All path args are sources, and the target dir was
            // specified separately
            target.clone()
        }
        None => {
            // If there was no explicit target-dir, then use the last
            // path_arg
            paths.pop().unwrap()
        }
    };

    // See comments on strip_trailing_slashes
    // if options.strip_trailing_slashes {
    //     for source in &mut paths {
    //         *source = source.components().as_path().to_owned();
    //     }
    // }

    Ok((paths, target))
}
fn determine_backup_mode(backup: String, span: Span) -> Result<BackupMode, ShellError> {
    match backup.as_str() {
        "simple" | "never" => Ok::<BackupMode, ShellError>(BackupMode::SimpleBackup),
        "numbered" | "t" => Ok(BackupMode::NumberedBackup),
        "existing" | "nil" => Ok(BackupMode::ExistingBackup),
        "none" | "off" => Ok(BackupMode::NoBackup),
        _ => Err(ShellError::GenericError(
            "Use `=` when using backup flag".into(),
            "Use `=` when using backup flag".into(),
            Some(span),
            None,
            Vec::new(),
        )),
    }
}

#[derive(Clone)]
pub struct Ucp;

impl Command for Ucp {
    fn name(&self) -> &str {
        "cp"
    }

    fn usage(&self) -> &str {
        "Copy files using uutils/coreutils cp."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["copy", "file", "files"]
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            // .required("source", SyntaxShape::GlobPattern, "the place to copy from")
            // .required("source", SyntaxShape::List(Box::new(SyntaxShape::Filepath)), "the place to copy from")
            // .required("destination", SyntaxShape::Filepath, "the place to copy to")
            .named(
                "target-directory",
                SyntaxShape::Filepath,
                "copy all SOURCE arguments into DIRECTORY",
                Some('t'),
            )
            .named(
                "reflink",
                SyntaxShape::String,
                "control clone/CoW copies. See below",
                None
                )
            .named(
                "sparse",
                SyntaxShape::String,
                "control creation of sparse files.",
                None
            )
            .switch("update", "copy only when the SOURCE file is newer than the destination file or when the destination file is missing", Some('u'))
            .named(
                "suffix",
                SyntaxShape::String,
                "override the usual backup suffix",
                Some('S')
            )
            .switch(
                "preserve",
                "preserve the default attributes (mode, owernship, timestamps)",
                Some('p')
                )
            .switch("backup", "make a backup of each existing destination file", Some('b'))
            .switch("recursive", "copy directories recursively", Some('r'))
            .switch("no-target-directory", "treat DEST as a normal file", Some('T'))
            .switch("verbose", "explicitly state what is being done", Some('v'))
            .switch("remove-destination", "remove each existing destination file before attempting to open it (contrast with --force)", None)
            .switch("force", "if an existing destination file cannot be opened, remove it and try again (this option is ignored when the -n option is also used). currently not implemented for windows", Some('f'))
            .switch("interactive", "ask before overwriting files", Some('i'))
            .switch("progress", "display a progress bar", Some('g'))
            .switch("no-clobber", "do not overwrite an existing file", Some('n'))
            .switch("symbolic-link", "make symbolic links instead of copying", Some('s'))
            // .switch("one-file-system", "stay on this file system", Some('x'))
            .switch("strip-trailing-slashes", "remove any trailing slashes from each SOURCE argument", None)
            // Make this the experimental GNU FLAG which has no long flag but we
            // provide one
            .switch("cli-symbolic-links", "follow command-line symbolic links in SOURCE", Some('H'))
            .switch("parents", "use full source file name under DIRECTORY", None)
            .switch("dereference", "dereference all symbolic links", Some('L'))
            .switch("no-dereference", "never follow symbolic links in SOURCE", Some('P'))
            .switch("no-dereference-preserve-links", "same as --no-dereference --preserve=links", Some('d'))
            // .switch("preserve-default-attributes", "same as --preserve=mode,ownership(unix only),timestamps", Some('p'))
            .switch("link", "hard-link files instead of copying", Some('l'))
            .switch("debug", "explain how a file is copied. Implies -v", None)
            .switch("copy-contents", "copy contents of special files when recursive", None)
            // .switch("archive", "same as -dR --preserve=all", Some('a'))
            .switch("attributes-only", "don't copy the file data, just the attributes", None)
            .rest("paths", SyntaxShape::Filepath, "the place to copy to")
            .category(Category::FileSystem)
    }

    fn examples(&self) -> Vec<Example> {
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
                example: "cp -u a b",
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
        // MVP args

        // [OPTIONS] SOURCE DEST
        // -r, --recursive     copy directories recursively [short aliases: R]
        // -v, --verbose       explicitly state what is being done (also adds --debug)
        // -f, --force         if an existing destination file cannot be opened, remove it and try again
        //                     (this option is ignored when the -n option is also used). Currently not
        //                     implemented for Windows.
        // -i, --interactive   ask before overwriting files
        // -g, --progress      Display a progress bar.
        // -b, --backup make a backup of.... technically it does not take an argument
        // but I think we can make it the same usage (not the like --backup but does not accept an
        // argument)
        // -l, --link,
        // -L, --dereference
        // -P, --no-dereference
        // -s, --symbolic-link,
        // -S, --sufix=SUFFIX,
        // -t, --target-directory=DIRECTORY
        // -T, --no-target-directory
        // -u, --update,
        // -x, --one-file-system
        // None, --remove-destination
        // NONE, --parents
        //None, --copy-contents

        let cli_dereference = call.has_flag("cli-symbolic-links");
        let attributes_only = call.has_flag("attributes-only");
        let link = call.has_flag("link");
        let symbolic_link = call.has_flag("symbolic-link");
        // let update = call.get_flag_expr("update");
        let update_mode = if call.has_flag("update") {
            UpdateMode::ReplaceIfOlder
        } else {
            UpdateMode::ReplaceAll
        };

        // let archive = call.has_flag("archive");
        let no_dereference_preserve_links = call.has_flag("no-dereference-preserve-links");
        let no_dereference = call.has_flag("no-dereference");
        // let preserve_default_attributes = call.has_flag("preserve-default-attributes");
        // dbg!(&update);
        let copy_mode = if link {
            uu_cp::CopyMode::Link
        } else if symbolic_link {
            uu_cp::CopyMode::SymLink
        } else if call.has_flag("update") {
            uu_cp::CopyMode::Update
        } else {
            uu_cp::CopyMode::Copy
        };
        let attributes = if call.has_flag("preserve") {
            // wonder if archive will be supported,
            // for now lets assume it wont be so either default or None
            uu_cp::Attributes::DEFAULT
        } else {
            uu_cp::Attributes::NONE
        };
        let dereference = call.has_flag("dereference");
        let force = call.has_flag("force");
        let interactive = call.has_flag("interactive");
        let no_target_dir = call.has_flag("no-target-directory");
        let no_clobber = call.has_flag("no-clobber");
        // let one_file_system = call.has_flag("one-file-system");
        let parents = call.has_flag("parents");
        let progress = call.has_flag("progress");
        let recursive = call.has_flag("recursive");
        let verbose = call.has_flag("verbose");

        let remove_destination = call.has_flag("remove-destination");
        let copy_contents = call.has_flag("copy-contents");
        // let backup_mode = call.get_flag_expr("backup");
        let backup = call.has_flag("backup");
        let reflink_mode = call.get_flag_expr("reflink");
        let debug = call.has_flag("debug");
        let sparse_mode = call.get_flag_expr("sparse");
        let strip_trailing_slashes = call.has_flag("strip-trailing-slashes");
        let suffix = call.get_flag_expr("suffix");
        let suffix = if let Some(suffix) = suffix {
            match suffix.expr {
                Expr::String(suffix) => Ok(suffix),
                _ => Err(ShellError::GenericError(
                    "Invalid type".into(),
                    "Invalid type".into(),
                    Some(suffix.span),
                    None,
                    Vec::new(),
                )),
            }
        } else {
            Ok(std::env::var("SIMPLE_BACKUP_SUFFIX").unwrap_or_else(|_| "~".to_owned()))
        }?;

        let sparse_mode = if let Some(sparse_mode) = sparse_mode {
            match sparse_mode.expr {
                Expr::String(sparse) => match sparse.as_str() {
                    "auto" => Ok(uu_cp::SparseMode::Auto),
                    "always" => Ok(uu_cp::SparseMode::Always),
                    "never" => Ok(uu_cp::SparseMode::Never),
                    _ => Err(ShellError::GenericError(
                        "Use `=` when using sparse flag".into(),
                        "Use `=` when using sparse flag".into(),
                        Some(sparse_mode.span),
                        None,
                        Vec::new(),
                    )),
                },
                _ => Err(ShellError::GenericError(
                    "Invalid type".into(),
                    "Invalid type".into(),
                    Some(sparse_mode.span),
                    None,
                    Vec::new(),
                )),
            }
        } else {
            Ok(uu_cp::SparseMode::Auto)
        }?;
        let reflink_mode = if let Some(reflink_mode) = reflink_mode {
            match reflink_mode.expr {
                Expr::String(reflink) => match reflink.as_str() {
                    "auto" => Ok(uu_cp::ReflinkMode::Auto),
                    "always" => Ok(uu_cp::ReflinkMode::Always),
                    "never" => Ok(uu_cp::ReflinkMode::Never),
                    _ => Err(ShellError::GenericError(
                        "Use `=` when using reflink flag".into(),
                        "Use `=` when using reflink flag".into(),
                        Some(reflink_mode.span),
                        None,
                        Vec::new(),
                    )),
                },
                _ => Err(ShellError::GenericError(
                    "Invalid type".into(),
                    "Invalid type".into(),
                    Some(reflink_mode.span),
                    None,
                    Vec::new(),
                )),
            }
        } else {
            #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
            {
                Ok(uu_cp::ReflinkMode::Auto)
            }
            #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
            {
                Ok(uu_cp::ReflinkMode::Never)
            }
        }?;
        let backup_mode = if backup {
            BackupMode::ExistingBackup
        } else {
            if let Some(backup) = stack.get_env_var(engine_state, "VERSION_CONTROL") {
                determine_backup_mode(backup.as_string()?, backup.span()?)?;
            }
            BackupMode::NoBackup
        };
        let target_expr = call.get_flag_expr("target-directory");
        let target_dir = if let Some(target_expr) = &target_expr {
            match &target_expr.expr {
                Expr::Filepath(path) => {
                    let target_path = PathBuf::from(path);
                    if !target_path.is_dir() {
                        return Err(ShellError::GenericError(
                            "is not a directory".into(),
                            "is not a directory".into(),
                            Some(target_expr.span),
                            None,
                            Vec::new(),
                        ));
                    };
                    Some(target_path)
                }
                _ => {
                    return Err(ShellError::GenericError(
                        "Invalid type".into(),
                        "Invalid type".into(),
                        Some(target_expr.span),
                        None,
                        Vec::new(),
                    ))
                }
            }
        } else {
            None
        };
        let overwrite = if no_clobber {
            uu_cp::OverwriteMode::NoClobber
        } else if interactive {
            if force {
                uu_cp::OverwriteMode::Interactive(uu_cp::ClobberMode::Force)
            } else if remove_destination {
                uu_cp::OverwriteMode::Interactive(uu_cp::ClobberMode::RemoveDestination)
            } else {
                uu_cp::OverwriteMode::Interactive(uu_cp::ClobberMode::Standard)
            }
        } else if force {
            uu_cp::OverwriteMode::Clobber(uu_cp::ClobberMode::Force)
        } else if remove_destination {
            uu_cp::OverwriteMode::Clobber(uu_cp::ClobberMode::RemoveDestination)
        } else {
            uu_cp::OverwriteMode::Clobber(uu_cp::ClobberMode::Standard)
        };

        //Conflicting flags need to be handled. For now lets handle the ones that I can see
        //for now with uutils tests.
        // 1. --no-clobber and --backup
        // TODOS
        if no_clobber && backup {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "Incompatible flags: --no-clobber and --backup are mutually exclusive"
                    .to_string(),
                span: call.head,
            });
        }

        let current_dir_path = current_dir(engine_state, stack)?;
        let num_files = call
            .arguments
            .iter()
            .filter_map(|arg| match arg {
                Argument::Positional(expr) => Some(expr),
                _ => None,
            })
            .count();

        let mut paths: Vec<PathBuf> = vec![];
        for file_nr in 0..num_files {
            let path = call.req::<Spanned<String>>(engine_state, stack, file_nr)?;
            let mut path = {
                Spanned {
                    item: nu_utils::strip_ansi_string_unlikely(path.item),
                    span: path.span,
                }
            };
            // this is the target
            if file_nr == num_files - 1 {
                // IF only one source and one target, and if target is a directory
                let target = PathBuf::from(&path.item);
                if path.item.ends_with('/')
                    && !target.is_dir()
                    && !call.has_flag("target-directory")
                {
                    return Err(ShellError::GenericError(
                        "is not a directory".into(),
                        "is not a directory".into(),
                        Some(path.span),
                        None,
                        Vec::new(),
                    ));
                };
                paths.push(target);
            } else {
                // these are the sources
                let src = &mut path.item;
                if strip_trailing_slashes {
                    // Workaround here rather than in `parse_path_args` (as it was done originally
                    // in uu_cp because otherwise the trailing slash
                    // gets eaten up by the glob_with. Therefore, need this hack here
                    // Maybe this fails on windows?
                    if std::env::consts::OS == "windows" {
                        *src = src.trim_end_matches('\\').to_string();
                    } else {
                        *src = src.trim_end_matches('/').to_string();
                    }
                }
                match nu_glob::glob_with(src, GLOB_PARAMS) {
                    Ok(files) => {
                        let mut f = files.filter_map(Result::ok).collect::<Vec<PathBuf>>();
                        if f.is_empty() {
                            return Err(ShellError::FileNotFound(path.span));
                        }
                        paths.append(&mut f);
                    }
                    Err(e) => {
                        return Err(ShellError::GenericError(
                            e.to_string(),
                            "invalid pattern".to_string(),
                            Some(path.span),
                            None,
                            Vec::new(),
                        ))
                    }
                }
            }
        }
        let options = uu_cp::Options {
            attributes_only,
            backup: backup_mode,
            copy_contents,
            cli_dereference,
            copy_mode,
            // dereference,
            dereference: !(no_dereference || no_dereference_preserve_links || recursive)
                || dereference,
            no_target_dir,
            one_file_system: false,
            overwrite,
            parents,
            sparse_mode,
            strip_trailing_slashes,
            reflink_mode,
            // attributes,
            attributes,
            recursive,
            backup_suffix: suffix,
            target_dir,
            // target_dir: None,
            update: update_mode,
            debug,
            verbose: verbose || debug,
            progress_bar: progress,
        };

        // For enabling easy `crawl` of cp command, we need to strip current directory
        // as it looks like uu_cp takes relative directory for sources & target,
        // but nushell always resolves to the $CWD
        // again, kind of a hack
        let paths = paths
            .iter()
            .map(|path| match path.strip_prefix(&current_dir_path) {
                Ok(p) => {
                    if p.to_str()?.is_empty() {
                        Some(PathBuf::from("."))
                    } else {
                        Some(p.to_path_buf())
                    }
                }
                Err(_) => Some(path.to_path_buf()),
            })
            // I guess this is caught by nu, if not
            // we handle it here.
            .collect::<Option<Vec<PathBuf>>>()
            .ok_or(ShellError::GenericError(
                "Not valid UTF-8".to_string(),
                "Not valid UTF-8".to_string(),
                None,
                None,
                vec![],
            ))?;
        // This function below was taken directly from `uu_cp` to reuse
        // some of their logic when it came to parsing path arguments,
        // even tho we already did some of it in the `src, target` for loop above
        // Nice to combine efforts and make it nicer.

        // TODO: Fix this following the comments in draft PR by Terts
        let (sources, target) = parse_path_args(paths, &options)?;
        if let Err(error) = uu_cp::copy(&sources, &target, &options) {
            match error {
                // Error::NotAllFilesCopied is non-fatal, but the error
                // code should still be EXIT_ERR as does GNU cp
                uu_cp::Error::NotAllFilesCopied => {}
                // Else we caught a fatal bubbled-up error, log it to stderr
                // ShellError
                // _ => uucore::macros::show_error!("{}", error),
                _ => {
                    return Err(ShellError::GenericError(
                        format!("{}", error),
                        format!("{}", error),
                        None,
                        None,
                        Vec::new(),
                    ))
                }
            };
            uucore::error::set_exit_code(EXIT_ERR);
        }
        Ok(PipelineData::empty())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // static TEST_EXISTING_FILE: &str = "existing_file.txt";
    // static TEST_HELLO_WORLD_SOURCE: &str = "hello_world.txt";
    // static TEST_HELLO_WORLD_SOURCE_SYMLINK: &str = "hello_world.txt.link";
    // static TEST_HELLO_WORLD_DEST: &str = "copy_of_hello_world.txt";
    // static TEST_HELLO_WORLD_DEST_SYMLINK: &str = "copy_of_hello_world.txt.link";
    // static TEST_HOW_ARE_YOU_SOURCE: &str = "how_are_you.txt";
    // static TEST_HOW_ARE_YOU_DEST: &str = "hello_dir/how_are_you.txt";
    // static TEST_COPY_TO_FOLDER: &str = "hello_dir/";
    // static TEST_COPY_TO_FOLDER_FILE: &str = "hello_dir/hello_world.txt";
    // static TEST_COPY_FROM_FOLDER: &str = "hello_dir_with_file/";
    // static TEST_COPY_FROM_FOLDER_FILE: &str = "hello_dir_with_file/hello_world.txt";
    // static TEST_COPY_TO_FOLDER_NEW: &str = "hello_dir_new";
    // static TEST_COPY_TO_FOLDER_NEW_FILE: &str = "hello_dir_new/hello_world.txt";
    // #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
    // static TEST_MOUNT_COPY_FROM_FOLDER: &str = "dir_with_mount";
    // #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
    // static TEST_MOUNT_MOUNTPOINT: &str = "mount";
    // #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
    // static TEST_MOUNT_OTHER_FILESYSTEM_FILE: &str = "mount/DO_NOT_copy_me.txt";
    // #[cfg(unix)]
    // static TEST_NONEXISTENT_FILE: &str = "nonexistent_file.txt";

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Ucp {})
    }

    // #[test]
    // fn test_cp_cp() {
    //     let (at, mut ucmd) = at_and_ucmd!();
    //     // Invoke our binary to make the copy.
    //     ucmd.arg(TEST_HELLO_WORLD_SOURCE)
    //         .arg(TEST_HELLO_WORLD_DEST)
    //         .succeeds();

    //     // Check the content of the destination file that was copied.
    //     assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
    // }
}
