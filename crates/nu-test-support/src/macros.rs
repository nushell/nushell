/// Run a command in nu and get it's output
///
/// The `nu!` macro accepts a number of options like the `cwd` in which the
/// command should be run. It is also possible to specify a different `locale`
/// to test locale dependent commands.
///
/// Pass options as the first arguments in the form of `key_1: value_1, key_1:
/// value_2, ...`. The options are defined in the `NuOpts` struct inside the
/// `nu!` macro.
///
/// The command can be formatted using `{}` just like `println!` or `format!`.
/// Pass the format arguments comma separated after the command itself.
///
/// # Examples
///
/// ```no_run
/// # // NOTE: The `nu!` macro needs the `nu` binary to exist. The test are
/// # //       therefore only compiled but not run (thats what the `no_run` at
/// # //       the beginning of this code block is for).
/// #
/// use nu_test_support::nu;
///
/// let outcome = nu!(
///     "date now | date to-record | get year"
/// );
///
/// let dir = "/";
/// let outcome = nu!(
///     "ls {} | get name",
///     dir,
/// );
///
/// let outcome = nu!(
///     cwd: "/",
///     "ls | get name",
/// );
///
/// let cell = "size";
/// let outcome = nu!(
///     locale: "de_DE.UTF-8",
///     "ls | into int {}",
///     cell,
/// );
///
/// let decimals = 2;
/// let outcome = nu!(
///     locale: "de_DE.UTF-8",
///     "10 | into string --decimals {}",
///     decimals,
/// );
/// ```
#[macro_export]
macro_rules! nu {
    // In the `@options` phase, we restucture all the
    // `$field_1: $value_1, $field_2: $value_2, ...`
    // pairs to a structure like
    // `@options[ $field_1 => $value_1 ; $field_2 => $value_2 ; ... ]`.
    // We do this to later distinguish the options from the `$path` and `$part`s.
    // (See
    //   https://users.rust-lang.org/t/i-dont-think-this-local-ambiguity-when-calling-macro-is-ambiguous/79401?u=x3ro
    // )
    //
    // If there is any special treatment needed for the `$value`, we can just
    // match for the specific `field` name.
    (
        @options [ $($options:tt)* ]
        cwd: $value:expr,
        $($rest:tt)*
    ) => {
        nu!(@options [ $($options)* cwd => $crate::fs::in_directory($value) ; ] $($rest)*)
    };
    // For all other options, we call `.into()` on the `$value` and hope for the best. ;)
    (
        @options [ $($options:tt)* ]
        $field:ident : $value:expr,
        $($rest:tt)*
    ) => {
        nu!(@options [ $($options)* $field => $value.into() ; ] $($rest)*)
    };

    // When the `$field: $value,` pairs are all parsed, the next tokens are the `$path` and any
    // number of `$part`s, potentially followed by a trailing comma.
    (
        @options [ $($options:tt)* ]
        $path:expr
        $(, $part:expr)*
        $(,)*
    ) => {{
        // Here we parse the options into a `NuOpts` struct
        let opts = nu!(@nu_opts $($options)*);
        // and format the `$path` using the `$part`s
        let path = nu!(@format_path $path, $($part),*);
        // Then finally we go to the `@main` phase, where the actual work is done.
        nu!(@main opts, path)
    }};

    // Create the NuOpts struct from the `field => value ;` pairs
    (@nu_opts $( $field:ident => $value:expr ; )*) => {
        NuOpts{
            $(
                $field: Some($value),
            )*
            ..Default::default()
        }
    };

    // Helper to format `$path`.
    (@format_path $path:expr $(,)?) => {
        // When there are no `$part`s, do not format anything
        $path
    };
    (@format_path $path:expr, $($part:expr),* $(,)?) => {{
        format!($path, $( $part ),*)
    }};

    // Do the actual work.
    (@main $opts:expr, $path:expr) => {{
        pub use std::error::Error;
        pub use std::io::prelude::*;
        pub use std::process::{Command, Stdio};
        pub use $crate::NATIVE_PATH_ENV_VAR;

        pub fn escape_quote_string(input: String) -> String {
            let mut output = String::with_capacity(input.len() + 2);
            output.push('"');

            for c in input.chars() {
                if c == '"' || c == '\\' {
                    output.push('\\');
                }
                output.push(c);
            }

            output.push('"');
            output
        }

        let test_bins = $crate::fs::binaries();

        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        let test_bins = nu_path::canonicalize_with(&test_bins, cwd).unwrap_or_else(|e| {
            panic!(
                "Couldn't canonicalize dummy binaries path {}: {:?}",
                test_bins.display(),
                e
            )
        });

        let mut paths = $crate::shell_os_paths();
        paths.insert(0, test_bins);

        let path = $path.lines().collect::<Vec<_>>().join("; ");

        let paths_joined = match std::env::join_paths(paths) {
            Ok(all) => all,
            Err(_) => panic!("Couldn't join paths for PATH var."),
        };

        let target_cwd = $opts.cwd.unwrap_or(".".to_string());
        let locale = $opts.locale.unwrap_or("en_US.UTF-8".to_string());

        let mut command = Command::new($crate::fs::executable_path());
        command
            .env("PWD", &target_cwd)
            .env(nu_utils::locale::LOCALE_OVERRIDE_ENV_VAR, locale)
            .current_dir(target_cwd)
            .env(NATIVE_PATH_ENV_VAR, paths_joined)
            // .arg("--skip-plugins")
            // .arg("--no-history")
            // .arg("--config-file")
            // .arg($crate::fs::DisplayPath::display_path(&$crate::fs::fixtures().join("playground/config/default.toml")))
            .arg(format!("-c {}", escape_quote_string(path)))
            .stdout(Stdio::piped())
            // .stdin(Stdio::piped())
            .stderr(Stdio::piped());

        let mut process = match command.spawn()
        {
            Ok(child) => child,
            Err(why) => panic!("Can't run test {:?} {}", $crate::fs::executable_path(), why.to_string()),
        };

        // let stdin = process.stdin.as_mut().expect("couldn't open stdin");
        // stdin
        //     .write_all(b"exit\n")
        //     .expect("couldn't write to stdin");

        let output = process
            .wait_with_output()
            .expect("couldn't read from stdout/stderr");

        let out = $crate::macros::read_std(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);

            println!("=== stderr\n{}", err);

        $crate::Outcome::new(out,err.into_owned())
    }};

    // This is the entrypoint for this macro.
    ($($token:tt)*) => {{
        #[derive(Default)]
        struct NuOpts {
            cwd: Option<String>,
            locale: Option<String>,
        }

        nu!(@options [ ] $($token)*)
    }};
}

#[macro_export]
macro_rules! with_exe {
    ($name:literal) => {{
        #[cfg(windows)]
        {
            concat!($name, ".exe")
        }
        #[cfg(not(windows))]
        {
            $name
        }
    }};
}

#[macro_export]
macro_rules! nu_with_plugins {
    (cwd: $cwd:expr, plugins: [$(($plugin_name:expr)),+$(,)?], $command:expr) => {{
        nu_with_plugins!($cwd, [$(("", $plugin_name)),+], $command)
    }};
    (cwd: $cwd:expr, plugin: ($plugin_name:expr), $command:expr) => {{
        nu_with_plugins!($cwd, [("", $plugin_name)], $command)
    }};

    ($cwd:expr, [$(($format:expr, $plugin_name:expr)),+$(,)?], $command:expr) => {{
        pub use std::error::Error;
        pub use std::io::prelude::*;
        pub use std::process::{Command, Stdio};
        pub use tempfile::tempdir;
        pub use $crate::{NATIVE_PATH_ENV_VAR, with_exe};

        let test_bins = $crate::fs::binaries();
        let test_bins = nu_path::canonicalize_with(&test_bins, ".").unwrap_or_else(|e| {
            panic!(
                "Couldn't canonicalize dummy binaries path {}: {:?}",
                test_bins.display(),
                e
            )
        });

        let temp = tempdir().expect("couldn't create a temporary directory");
        let temp_plugin_file = temp.path().join("plugin.nu");
        std::fs::File::create(&temp_plugin_file).expect("couldn't create temporary plugin file");

        $($crate::commands::ensure_binary_present($plugin_name);)+

        // TODO: the `$format` is a dummy empty string, but `plugin_name` is repeatable
        // just keep it here for now.  Need to find a way to remove it.
        let registrations = format!(
            concat!($(concat!("register ", $format, " {};")),+),
            $(
                nu_path::canonicalize_with(with_exe!($plugin_name), &test_bins)
                    .unwrap_or_else(|e| {
                        panic!("failed to canonicalize plugin {} path", $plugin_name)
                    })
                    .display()
            ),+
        );
        let commands = format!("{registrations}{}", $command);

        let target_cwd = $crate::fs::in_directory(&$cwd);
        let mut process = match Command::new($crate::fs::executable_path())
            .current_dir(&target_cwd)
            .env("PWD", &target_cwd) // setting PWD is enough to set cwd
            .arg("--commands")
            .arg(commands)
            .arg("--plugin-config")
            .arg(temp_plugin_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(why) => panic!("Can't run test {}", why.to_string()),
        };

        let output = process
            .wait_with_output()
            .expect("couldn't read from stdout/stderr");

        let out = $crate::macros::read_std(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);

        println!("=== stderr\n{}", err);

        $crate::Outcome::new(out, err.into_owned())
    }};
}

pub fn read_std(std: &[u8]) -> String {
    let out = String::from_utf8_lossy(std);
    let out = out.lines().collect::<Vec<_>>().join("\n");
    let out = out.replace("\r\n", "");
    out.replace('\n', "")
}
