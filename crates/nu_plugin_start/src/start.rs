use nu_protocol::CallInfo;
use nu_protocol::Value;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct Start {
    pub filenames: Vec<String>,
    pub application: Option<String>,
}

impl Start {
    pub fn parse(&mut self, call_info: CallInfo) {
        self.parse_filenames(&call_info);
        self.parse_application(&call_info);
    }

    fn parse_filenames(&mut self, call_info: &CallInfo) {
        let candidates = match &call_info.args.positional {
            Some(values) => values
                .iter()
                .map(|val| val.as_string())
                .collect::<Result<Vec<String>, _>>()
                .unwrap_or(vec![]),
            None => vec![],
        };
        println!("{:?}", candidates);
    }

    fn parse_application(&mut self, call_info: &CallInfo) {
        self.application = if let Some(app) = call_info.args.get("application") {
            match app.as_string() {
                Ok(name) => Some(name),
                Err(_) => None,
            }
        } else {
            None
        };
    }

    pub fn add_filename(&mut self, input: &Value) {
        if let Ok(filename) = input.as_string() {
            if Path::new(&filename).exists() {
                self.filenames.push(filename);
            }
        } else {
            // print warning that filename doesn't exist
            println!("doesn't exist");
        }
    }

    // pub fn exec(call_info: &CallInfo) -> Result<(), String> {
    //     let application = if let Some(app) = call_info.args.get("application") {
    //         match app.as_string() {
    //             Ok(name) => Some(name),
    //             Err(_) => return Err(String::from("Application name not found")),
    //         }
    //     } else {
    //         None
    //     };
    //     // let filenames = Self::filenames(&call_info);
    //     // check if the files exist
    //     for file in filenames.iter() {
    //         if !Path::new(file.as_str()).exists() && url::Url::parse(file).is_err() {
    //             return Err(format!("The file '{}' could not be found", file));
    //         }
    //     }
    //     Self::run(filenames, application)
    // }

    // #[cfg(target_os = "macos")]
    // fn run(mut filenames: Vec<String>, application: Option<String>) -> Result<(), String> {
    //     let mut args = vec![];
    //     args.append(&mut filenames);
    //     if let Some(name) = application {
    //         args.append(&mut vec![String::from("-a"), name]);
    //     }

    //     Command::new("open")
    //         .stdout(Stdio::null())
    //         .stderr(Stdio::null())
    //         .args(&args)
    //         .status();

    //     Ok(())
    // }
}
