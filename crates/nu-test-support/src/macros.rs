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
        pub use std::error::Error;
        pub use std::io::prelude::*;
        pub use std::process::{Command, Stdio};

        let commands = &*format!(
            "
                            cd {}
                            {}
                            exit",
            $crate::fs::in_directory($cwd),
            $crate::fs::DisplayPath::display_path(&$path)
        );

        let dummies = $crate::fs::binaries();
        let dummies = dunce::canonicalize(&dummies).unwrap_or_else(|e| {
            panic!(
                "Couldn't canonicalize dummy binaries path {}: {:?}",
                dummies.display(),
                e
            )
        });

        let mut process = match Command::new($crate::fs::executable_path())
            // .env_clear()
            .env("PATH", dummies)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(why) => panic!("Can't run test {}", why.to_string()),
        };

        let stdin = process.stdin.as_mut().expect("couldn't open stdin");
        stdin
            .write_all(commands.as_bytes())
            .expect("couldn't write to stdin");

        let output = process
            .wait_with_output()
            .expect("couldn't read from stdout");

        let out = $crate::macros::read_std(&output.stdout);
        let err = $crate::macros::read_std(&output.stderr);

        println!("=== stderr\n{}", err);

        out
    }};
}

pub fn read_std(std: &[u8]) -> String {
    let out = String::from_utf8_lossy(std);
    let out = out.lines().skip(1).collect::<Vec<_>>().join("\n");
    let out = out.replace("\r\n", "");
    let out = out.replace("\n", "");
    out
}

#[macro_export]
macro_rules! nu_error {
    (cwd: $cwd:expr, $path:expr, $($part:expr),*) => {{
        use $crate::fs::DisplayPath;

        let path = format!($path, $(
            $part.display_path()
        ),*);

        nu_error!($cwd, &path)
    }};

    (cwd: $cwd:expr, $path:expr) => {{
        nu_error!($cwd, $path)
    }};

    ($cwd:expr, $path:expr) => {{
        pub use std::error::Error;
        pub use std::io::prelude::*;
        pub use std::process::{Command, Stdio};

        let commands = &*format!(
            "
                            cd {}
                            {}
                            exit",
            $crate::fs::in_directory($cwd),
            $crate::fs::DisplayPath::display_path(&$path)
        );

        let dummies = $crate::fs::binaries();
        let dummies = dunce::canonicalize(&dummies).unwrap_or_else(|e| {
            panic!(
                "Couldn't canonicalize dummy binaries path {}: {:?}",
                dummies.display(),
                e
            )
        });

        let mut process = Command::new($crate::fs::executable_path())
            // .env_clear()
            .env("PATH", dummies)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("couldn't run test");

        let stdin = process.stdin.as_mut().expect("couldn't open stdin");
        stdin
            .write_all(commands.as_bytes())
            .expect("couldn't write to stdin");

        let output = process
            .wait_with_output()
            .expect("couldn't read from stdout/stderr");

        let out = String::from_utf8_lossy(&output.stderr);
        out.into_owned()
    }};
}
