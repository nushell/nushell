use nu_protocol::CallInfo;
use std::process::{Command, Stdio};

pub struct Start;

impl Start {
    fn filenames(call_info: &CallInfo) -> Vec<String> {
        match &call_info.args.positional {
            Some(values) => values
                .iter()
                .map(|val| val.as_string())
                .collect::<Result<Vec<String>, _>>()
                .unwrap_or(vec![]),
            None => vec![],
        }
    }

    #[cfg(target_os = "macos")]
    pub fn exec(call_info: &CallInfo) -> Result<(), &str> {
        // let mut command;
        let mut args = vec![];

        if let Some(app) = call_info.args.get("application") {
            match app.as_string() {
                Ok(name) => args.append(&mut vec![String::from("-a"), name]),
                Err(_) => return Err("Application name not found"),
            }
        }

        args.append(&mut Self::filenames(&call_info));
        Command::new("open")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .args(&args)
            .spawn();

        Ok(())
    }
}
