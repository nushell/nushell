use crate::Host;
use nu_protocol::{errln, outln};
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

    #[allow(unused_variables)]
    fn vars(&mut self) -> Vec<(String, String)> {
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

    fn out_termcolor(&self) -> termcolor::StandardStream {
        termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto)
    }

    fn err_termcolor(&self) -> termcolor::StandardStream {
        termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto)
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
}
