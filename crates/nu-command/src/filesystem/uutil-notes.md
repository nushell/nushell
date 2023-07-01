# Using uu_cratename in nushell

## Argument Passing

We need to get arguments form nushell and pass arguments to uumain. I seriously doubt that the nushell argument structures will be 100% compatible with clap, which is what is used in uutils, so there could be some rough edges.

This is a prototype for copy in PR #9463

```rust
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

    // Create uucore::Args somehow from nushell args
    let s1 = "cp".to_string();
    let s2 = "-h".to_string();
    let args = vec![OsString::from(s1), OsString::from(s2)];
    // Pass uucore::Args to app.uumain
    uu_cp::uumain(&mut args.into_iter());
    Ok(PipelineData::empty())
}
```

You can see that I'm creating `s1` and `s2` just as strings to pass as arguments into `uumain`. We could do this with only the arguments we want to support in nushell. Maybe we start with a few popular arguments and grow them as needed.

- Another approach is to entirely creat our own main. This is what the `uu_cp` `uumain` currently looks like.

```rust
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args);

    // The error is parsed here because we do not want version or help being printed to stderr.
    if let Err(e) = matches {
        let mut app = uu_app();

        match e.kind() {
            clap::error::ErrorKind::DisplayHelp => {
                app.print_help()?;
            }
            clap::error::ErrorKind::DisplayVersion => print!("{}", app.render_version()),
            _ => return Err(Box::new(e.with_exit_code(1))),
        };
    } else if let Ok(mut matches) = matches {
        let options = Options::from_matches(&matches)?;

        if options.overwrite == OverwriteMode::NoClobber && options.backup != BackupMode::NoBackup {
            return Err(UUsageError::new(
                EXIT_ERR,
                "options --backup and --no-clobber are mutually exclusive",
            ));
        }

        let paths: Vec<PathBuf> = matches
            .remove_many::<PathBuf>(options::PATHS)
            .map(|v| v.collect())
            .unwrap_or_default();

        let (sources, target) = parse_path_args(paths, &options)?;

        if let Err(error) = copy(&sources, &target, &options) {
            match error {
                // Error::NotAllFilesCopied is non-fatal, but the error
                // code should still be EXIT_ERR as does GNU cp
                Error::NotAllFilesCopied => {}
                // Else we caught a fatal bubbled-up error, log it to stderr
                _ => show_error!("{}", error),
            };
            set_exit_code(EXIT_ERR);
        }
    }

    Ok(())
}
```

We could call `let matches = uu_app().try_get_matches_from(args);` from our nushell command run function and return `ShellError` on parameter issues. So, the nushell run function for cp could look something more familiar.

```rust
fn run(
    &self,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    // Do our normal nushell argument parsing

    // Create uucore::Args somehow from nushell args
    // Seems like the easiest way for copy is just to use OsStrong::from()
    let s1 = "cp".to_string();
    let s2 = "-h".to_string();
    let args = vec![OsString::from(s1), OsString::from(s2)];

    // Get the App
    let matches = uu_app().try_get_matches_from(args);
    // We match on matches.
    //
    //If things go the happy path, we call
    // `copy(&source, &target, &options)`
    // Which means `copy` has to be exposed to us, as well as the
    // source, target, and copy Options struct.
    //
    // If we go the unhappy path, we have to do error handling.
    // I doubt we have spans here. So, having nice errors may not be
    // possible yet. I wonder if clap can return spans somewhere?


    // All the above code is literally reimplemnting `uu_cp::uumain()`
    // really for the sole purpose of nushell error handling and
    // error reporting
    //
    // Pass uucore::Args to app.uumain
    // uu_cp::uumain(&mut args.into_iter());

    Ok(PipelineData::empty())
}
```
