#[macro_export]
macro_rules! nu {
    (cwd: $cwd:expr, $path:expr, $($part:expr),*) => {{
        use $crate::fs::DisplayPath;

        let path = format!($path, $(
            $part.display_path()
        ),*);

        nu!($cwd, &path)
    }};

    (cwd: $cwd:expr, $path:expr) => {{
        nu!($cwd, $path)
    }};

    ($cwd:expr, $path:expr) => {{
        pub use itertools::Itertools;
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

        // let commands = &*format!(
        //     "
        //                     {}
        //                     exit",
        //     $crate::fs::DisplayPath::display_path(&$path)
        // );

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

        let target_cwd = $crate::fs::in_directory(&$cwd);

        let mut process = match Command::new($crate::fs::executable_path())
            .env("PWD", &target_cwd)
            .current_dir(target_cwd)
            .env(NATIVE_PATH_ENV_VAR, paths_joined)
            // .arg("--skip-plugins")
            // .arg("--no-history")
            // .arg("--config-file")
            // .arg($crate::fs::DisplayPath::display_path(&$crate::fs::fixtures().join("playground/config/default.toml")))
            .arg(format!("-c {}", escape_quote_string($crate::fs::DisplayPath::display_path(&path))))
            .stdout(Stdio::piped())
            // .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
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
    (cwd: $cwd:expr, plugins: [$(($format:expr, $plugin_name:expr)),+$(,)?], $command:expr) => {{
        nu_with_plugins!($cwd, [$(($format, $plugin_name)),+], $command)
    }};
    (cwd: $cwd:expr, plugin: ($format:expr, $plugin_name:expr), $command:expr) => {{
        nu_with_plugins!($cwd, [($format, $plugin_name)], $command)
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

        let registrations = format!(
            concat!($(concat!("register -e ", $format, " {};")),+),
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
