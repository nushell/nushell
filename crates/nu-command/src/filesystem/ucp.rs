use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use std::ffi::OsString;
use std::path::PathBuf;

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
            .required("source", SyntaxShape::GlobPattern, "the place to copy from")
            .required("destination", SyntaxShape::Filepath, "the place to copy to")
            .switch("recursive", "copy directories recursively", Some('r'))
            .switch("verbose", "explicitly state what is being done", Some('v'))
            .switch("force", "if an existing destination file cannot be opened, remove it and try again (this option is ignored when the -n option is also used). currently not implemented for windows", Some('f'))
            .switch("interactive", "ask before overwriting files", Some('i'))
            .switch("progress", "display a progress bar", Some('g'))
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
                description: "Copy many file recursively into a new folder showing a progress bar",
                example: "cp -r -g big_folder new_folder",
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
        // Get the App
        // let mut app = uu_cp::uu_app();
        // app.print_help()?;

        // MVP args
        // [OPTIONS] SOURCE DEST
        // -r, --recursive     copy directories recursively [short aliases: R]
        // -v, --verbose       explicitly state what is being done (also adds --debug)
        // -f, --force         if an existing destination file cannot be opened, remove it and try again
        //                     (this option is ignored when the -n option is also used). Currently not
        //                     implemented for Windows.
        // -i, --interactive   ask before overwriting files
        // -g, --progress      Display a progress bar.
        let src: Spanned<String> = call.req(engine_state, stack, 0)?;
        let src = {
            Spanned {
                item: nu_utils::strip_ansi_string_unlikely(src.item),
                span: src.span,
            }
        };
        let dst: Spanned<String> = call.req(engine_state, stack, 1)?;
        let recursive = call.has_flag("recursive");
        let verbose = call.has_flag("verbose");
        let interactive = call.has_flag("interactive");
        let progress = call.has_flag("progress");

        let current_dir_path = current_dir(engine_state, stack)?;
        let source = current_dir_path.join(src.item.as_str());
        let destination = current_dir_path.join(dst.item.as_str());
        // let ctrlc = engine_state.ctrlc.clone();
        // let span = call.head;

        // POC
        // Create uucore::Args somehow from nushell args
        // let s1 = "cp".to_string();
        // let s2 = "-h".to_string();
        // let args = vec![OsString::from(s1), OsString::from(s2)];

        let mut args: Vec<OsString> = vec!["cp".into()]; // seed it with the cp command
        if recursive {
            // working
            args.push("-r".into());
        }
        if verbose {
            // working
            args.push("-v".into());
            args.push("--debug".into());
        }
        if interactive {
            // working
            args.push("-i".into());
        }
        if progress {
            // working (you won't see it unless there are a lot of files)
            args.push("-g".into());
        }
        let src_input = match source.to_str() {
            Some(s) => s,
            None => {
                return Err(ShellError::GenericError(
                    "No source input provided".into(),
                    "No source input provided".into(),
                    Some(src.span),
                    None,
                    Vec::new(),
                ))
            }
        };
        let dest_input = match destination.to_str() {
            Some(d) => d,
            None => {
                return Err(ShellError::GenericError(
                    "No destination input provided".into(),
                    "No destination input provided".into(),
                    Some(dst.span),
                    None,
                    Vec::new(),
                ))
            }
        };

        args.push(src_input.into());
        args.push(dest_input.into());

        // Pass uucore::Args to app.uumain
        let hidden_options = true;

        // If hidden_options is set to true, this means just call the uumain function
        if hidden_options {
            uu_cp::uumain(args.into_iter());
        } else {
            use uucore::mods::backup_control::BackupMode;
            use uucore::mods::update_control::UpdateMode;

            // static EXIT_ERR: i32 = 1;

            // If hidden_options is set to false, this means we need to parse the args
            // ourselves and throw ShellErrors if bad things happen. We don't have spans yet though.
            // Also, changes have to be made to uucore and uu_cp to make this work. These
            // changes are really just making some enums, structs, and functions public.
            let attrs = uu_cp::Attributes::default();
            let _options = uu_cp::Options {
                attributes_only: false,
                backup: BackupMode::SimpleBackup,
                copy_contents: true,
                cli_dereference: true,
                copy_mode: uu_cp::CopyMode::Copy,
                dereference: true,
                no_target_dir: false,
                one_file_system: false,
                overwrite: uu_cp::OverwriteMode::NoClobber,
                parents: false,
                sparse_mode: uu_cp::SparseMode::Auto,
                strip_trailing_slashes: true,
                reflink_mode: uu_cp::ReflinkMode::Auto,
                attributes: attrs,
                recursive,
                backup_suffix: "bak".to_string(),
                target_dir: Some(destination),
                update: UpdateMode::ReplaceIfOlder,
                debug: true,
                verbose,
                progress_bar: progress,
            };

            // Try to use uu error handling but make the errors ShellError
            // I think this is the only way to catch errors from uu and
            // turn them in to ShellError.
            //
            // We'd have to iterat through uu_app().try_get_matches_from(args)
            // and when there are errors, create a ShellError. However, for
            // this to work, it seems like we need a special mode for nushell
            // to pass parameters through without checking them.
            //
            // Once we pass that hurdle, we'll need some things in uu to be
            // pub like uu_cp::Options (any_uu_cmd::Options) will probably
            // need to be public to go this route.
            //
            // After options, we'll need the commands themselves to be public
            // such as uu_cp::copy() and uu_cp::parse_path_args()

            let matches = uu_cp::uu_app().try_get_matches_from(args);
            // eprint!("Result<ArgMatches>: {:#?}\n\n", matches);

            // The error is parsed here because we do not want version or help being printed to stderr.
            if let Err(_e) = matches {
                // We don't get here because nushell pukes on bad params.
                // We do need to figure out how to support all the syntax that cp/mv support
                // like mv [a b c d] test <-- this doesn't work yet
                let mut app = uu_cp::uu_app();
                app.print_help()?;
            } else if let Ok(mut matches) = matches {
                if let Ok(options) = uu_cp::Options::from_matches(&matches) {
                    eprintln!("{:#?}\n\n", options);
                    // eprint!("ArgMatches: {:#?}\n\n", matches);

                    if let Some(c) = matches.get_one::<bool>("recursive") {
                        println!("Value for recursive(-r): {c}");
                    }
                    if let Some(c) = matches.get_one::<bool>("progress") {
                        println!("Value for progress(-g): {c}");
                    }
                    if let Some(c) = matches.get_one::<bool>("verbose") {
                        println!("Value for verbose(-v): {c}");
                    }
                    if let Some(c) = matches.get_one::<bool>("force") {
                        println!("Value for force(-f): {c}");
                    }
                    if let Some(c) = matches.get_one::<bool>("interactive") {
                        println!("Value for interactive(-i): {c}");
                    }

                    if options.overwrite == uu_cp::OverwriteMode::NoClobber
                        && options.backup != BackupMode::NoBackup
                    {
                        return Err(ShellError::IncompatibleParametersSingle {
                            msg: "options --backup and --no-clobber are mutually exclusive"
                                .to_string(),
                            span: call.head,
                        });
                    }

                    let paths: Vec<PathBuf> = matches
                        .remove_many::<PathBuf>(uu_cp::options::PATHS)
                        .map(|v| v.collect())
                        .unwrap_or_default();

                    eprintln!("paths: {:?}", paths);

                    let (sources, target) =
                        uu_cp::parse_path_args(paths, &options).map_err(|error| {
                            ShellError::GenericError(
                                format!("{}", error),
                                format!("{}", error),
                                None,
                                None,
                                Vec::new(),
                            )
                        })?;

                    if let Err(error) = uu_cp::copy(&sources, &target, &options) {
                        match error {
                            // Error::NotAllFilesCopied is non-fatal, but the error
                            // code should still be EXIT_ERR as does GNU cp
                            // Error::NotAllFilesCopied => {}
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
                        // uucore::error::set_exit_code(EXIT_ERR);
                    }
                }
            }
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
