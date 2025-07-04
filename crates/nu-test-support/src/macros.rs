/// Run a command in nu and get its output
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
/// # //       therefore only compiled but not run (that's what the `no_run` at
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
        $crate::macros::NuOpts{
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
        $crate::macros::nu_run_test($opts, $path, false)
    }};

    // This is the entrypoint for this macro.
    ($($token:tt)*) => {{

        nu!(@options [ ] $($token)*)
    }};
}

#[macro_export]
macro_rules! nu_with_std {
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
        nu_with_std!(@options [ $($options)* cwd => $crate::fs::in_directory($value) ; ] $($rest)*)
    };
    // For all other options, we call `.into()` on the `$value` and hope for the best. ;)
    (
        @options [ $($options:tt)* ]
        $field:ident : $value:expr,
        $($rest:tt)*
    ) => {
        nu_with_std!(@options [ $($options)* $field => $value.into() ; ] $($rest)*)
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
        let opts = nu_with_std!(@nu_opts $($options)*);
        // and format the `$path` using the `$part`s
        let path = nu_with_std!(@format_path $path, $($part),*);
        // Then finally we go to the `@main` phase, where the actual work is done.
        nu_with_std!(@main opts, path)
    }};

    // Create the NuOpts struct from the `field => value ;` pairs
    (@nu_opts $( $field:ident => $value:expr ; )*) => {
        $crate::macros::NuOpts{
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
        $crate::macros::nu_run_test($opts, $path, true)
    }};

    // This is the entrypoint for this macro.
    ($($token:tt)*) => {{
        nu_with_std!(@options [ ] $($token)*)
    }};
}

#[macro_export]
macro_rules! nu_with_plugins {
    (cwd: $cwd:expr, plugins: [$(($plugin_name:expr)),*$(,)?], $command:expr) => {{
        nu_with_plugins!(
            cwd: $cwd,
            envs: Vec::<(&str, &str)>::new(),
            plugins: [$(($plugin_name)),*],
            $command
        )
    }};
    (cwd: $cwd:expr, plugin: ($plugin_name:expr), $command:expr) => {{
        nu_with_plugins!(
            cwd: $cwd,
            envs: Vec::<(&str, &str)>::new(),
            plugin: ($plugin_name),
            $command
        )
    }};

    (
        cwd: $cwd:expr,
        envs: $envs:expr,
        plugins: [$(($plugin_name:expr)),*$(,)?],
        $command:expr
    ) => {{
        $crate::macros::nu_with_plugin_run_test($cwd, $envs, &[$($plugin_name),*], $command)
    }};
    (cwd: $cwd:expr, envs: $envs:expr, plugin: ($plugin_name:expr), $command:expr) => {{
        $crate::macros::nu_with_plugin_run_test($cwd, $envs, &[$plugin_name], $command)
    }};

}

use crate::{NATIVE_PATH_ENV_VAR, Outcome};
use nu_path::{AbsolutePath, AbsolutePathBuf, Path, PathBuf};
use nu_utils::escape_quote_string;
use std::{
    ffi::OsStr,
    process::{Command, Stdio},
};
use tempfile::tempdir;

#[derive(Default)]
pub struct NuOpts {
    pub cwd: Option<AbsolutePathBuf>,
    pub locale: Option<String>,
    pub envs: Option<Vec<(String, String)>>,
    pub collapse_output: Option<bool>,
    // Note: At the time this was added, passing in a file path was more convenient. However,
    // passing in file contents seems like a better API - consider this when adding new uses of
    // this field.
    pub env_config: Option<PathBuf>,
}

pub fn nu_run_test(opts: NuOpts, commands: impl AsRef<str>, with_std: bool) -> Outcome {
    let test_bins = crate::fs::binaries()
        .canonicalize()
        .expect("Could not canonicalize dummy binaries path");

    let mut paths = crate::shell_os_paths();
    paths.insert(0, test_bins.into());

    let commands = commands.as_ref().lines().collect::<Vec<_>>().join("; ");

    let paths_joined = match std::env::join_paths(paths) {
        Ok(all) => all,
        Err(_) => panic!("Couldn't join paths for PATH var."),
    };

    let target_cwd = opts.cwd.unwrap_or_else(crate::fs::root);
    let locale = opts.locale.unwrap_or("en_US.UTF-8".to_string());
    let executable_path = crate::fs::executable_path();

    let mut command = setup_command(&executable_path, &target_cwd);
    command
        .env(nu_utils::locale::LOCALE_OVERRIDE_ENV_VAR, locale)
        .env(NATIVE_PATH_ENV_VAR, paths_joined);

    if let Some(envs) = opts.envs {
        command.envs(envs);
    }

    match opts.env_config {
        Some(path) => command.arg("--env-config").arg(path),
        // TODO: This seems unnecessary: the code that runs for integration tests
        // (run_commands) loads startup configs only if they are specified via flags explicitly or
        // the shell is started as logging shell (which it is not in this case).
        None => command.arg("--no-config-file"),
    };

    if !with_std {
        command.arg("--no-std-lib");
    }
    // Use plain errors to help make error text matching more consistent
    command.args(["--error-style", "plain"]);
    command
        .arg(format!("-c {}", escape_quote_string(&commands)))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Uncomment to debug the command being run:
    // println!("=== command\n{command:?}\n");

    let process = match command.spawn() {
        Ok(child) => child,
        Err(why) => panic!("Can't run test {:?} {}", crate::fs::executable_path(), why),
    };

    let output = process
        .wait_with_output()
        .expect("couldn't read from stdout/stderr");

    let out = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);

    let out = if opts.collapse_output.unwrap_or(true) {
        collapse_output(&out)
    } else {
        out.into_owned()
    };

    println!("=== stderr\n{err}");

    Outcome::new(out, err.into_owned(), output.status)
}

pub fn nu_with_plugin_run_test<E, K, V>(
    cwd: impl AsRef<Path>,
    envs: E,
    plugins: &[&str],
    command: &str,
) -> Outcome
where
    E: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let test_bins = crate::fs::binaries();
    let test_bins = nu_path::canonicalize_with(&test_bins, ".").unwrap_or_else(|e| {
        panic!(
            "Couldn't canonicalize dummy binaries path {}: {:?}",
            test_bins.display(),
            e
        )
    });

    let temp = tempdir().expect("couldn't create a temporary directory");
    let [temp_config_file, temp_env_config_file] = ["config.nu", "env.nu"].map(|name| {
        let temp_file = temp.path().join(name);
        std::fs::File::create(&temp_file).expect("couldn't create temporary config file");
        temp_file
    });

    // We don't have to write the plugin registry file, it's ok for it to not exist
    let temp_plugin_file = temp.path().join("plugin.msgpackz");

    crate::commands::ensure_plugins_built();

    let plugin_paths_quoted: Vec<String> = plugins
        .iter()
        .map(|plugin_name| {
            let plugin = with_exe(plugin_name);
            let plugin_path = nu_path::canonicalize_with(&plugin, &test_bins)
                .unwrap_or_else(|_| panic!("failed to canonicalize plugin {} path", &plugin));
            let plugin_path = plugin_path.to_string_lossy();
            escape_quote_string(&plugin_path)
        })
        .collect();
    let plugins_arg = format!("[{}]", plugin_paths_quoted.join(","));

    let target_cwd = crate::fs::in_directory(&cwd);
    // In plugin testing, we need to use installed nushell to drive
    // plugin commands.
    let mut executable_path = crate::fs::executable_path();
    if !executable_path.exists() {
        executable_path = crate::fs::installed_nu_path();
    }

    let process = match setup_command(&executable_path, &target_cwd)
        .envs(envs)
        .arg("--commands")
        .arg(command)
        // Use plain errors to help make error text matching more consistent
        .args(["--error-style", "plain"])
        .arg("--config")
        .arg(temp_config_file)
        .arg("--env-config")
        .arg(temp_env_config_file)
        .arg("--plugin-config")
        .arg(temp_plugin_file)
        .arg("--plugins")
        .arg(plugins_arg)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(why) => panic!("Can't run test {why}"),
    };

    let output = process
        .wait_with_output()
        .expect("couldn't read from stdout/stderr");

    let out = collapse_output(&String::from_utf8_lossy(&output.stdout));
    let err = String::from_utf8_lossy(&output.stderr);

    println!("=== stderr\n{err}");

    Outcome::new(out, err.into_owned(), output.status)
}

fn with_exe(name: &str) -> String {
    #[cfg(windows)]
    {
        name.to_string() + ".exe"
    }
    #[cfg(not(windows))]
    {
        name.to_string()
    }
}

fn collapse_output(out: &str) -> String {
    let out = out.lines().collect::<Vec<_>>().join("\n");
    let out = out.replace("\r\n", "");
    out.replace('\n', "")
}

fn setup_command(executable_path: &AbsolutePath, target_cwd: &AbsolutePath) -> Command {
    let mut command = Command::new(executable_path);

    command
        .current_dir(target_cwd)
        .env_remove("FILE_PWD")
        .env("PWD", target_cwd); // setting PWD is enough to set cwd;

    command
}
