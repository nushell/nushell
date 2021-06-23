use crate::Host;
use nu_errors::ShellError;
use nu_protocol::{errln, outln};
use nu_source::Text;
use std::ffi::OsString;

#[derive(Debug)]
pub struct BasicHost;

impl Host for BasicHost {
    fn stdout(&mut self, out: &str) {
        match out {
            "\n" => outln!(""),
            other => outln!("{}", other),
        }
    }

    fn stderr(&mut self, out: &str) {
        match out {
            "\n" => errln!(""),
            other => errln!("{}", other),
        }
    }

    fn print_err(&mut self, err: ShellError, source: &Text) {
        let diag = err.into_diagnostic();
        let source = source.to_string();
        let mut files = codespan_reporting::files::SimpleFiles::new();
        files.add("shell", source);

        let writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto);
        let config = codespan_reporting::term::Config::default();

        let _ = std::panic::catch_unwind(move || {
            let _ = codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &diag);
        });
    }

    #[allow(unused_variables)]
    fn vars(&self) -> Vec<(String, String)> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::vars().collect::<Vec<_>>()
        }

        #[cfg(target_arch = "wasm32")]
        {
            vec![]
        }
    }

    #[allow(unused_variables)]
    fn env_get(&mut self, key: OsString) -> Option<OsString> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::var_os(key)
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    #[allow(unused_variables)]
    fn env_set(&mut self, key: OsString, value: OsString) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::set_var(key, value);
        }
    }

    #[allow(unused_variables)]
    fn env_rm(&mut self, key: OsString) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::env::remove_var(key);
        }
    }

    fn width(&self) -> usize {
        let (mut term_width, _) = term_size::dimensions().unwrap_or((80, 20));
        term_width -= 1;
        term_width
    }

    fn height(&self) -> usize {
        let (_, term_height) = term_size::dimensions().unwrap_or((80, 20));
        term_height
    }

    fn is_external_cmd(&self, #[allow(unused)] cmd_name: &str) -> bool {
        #[cfg(any(target_arch = "wasm32", not(feature = "which")))]
        {
            true
        }

        #[cfg(all(unix, feature = "which"))]
        {
            which::which(cmd_name).is_ok()
        }

        #[cfg(all(windows, feature = "which"))]
        {
            if which::which(cmd_name).is_ok() {
                true
            } else {
                // Reference: https://ss64.com/nt/syntax-internal.html
                let cmd_builtins = [
                    "assoc", "break", "color", "copy", "date", "del", "dir", "dpath", "echo",
                    "erase", "for", "ftype", "md", "mkdir", "mklink", "move", "path", "ren",
                    "rename", "rd", "rmdir", "start", "time", "title", "type", "ver", "verify",
                    "vol",
                ];

                cmd_builtins.contains(&cmd_name)
            }
        }
    }
}
