use nu_cli::App as CliApp;
use nu_errors::ShellError;

fn main() -> Result<(), ShellError> {
    let mut argv = vec![String::from("nu")];
    argv.extend(positionals());

    CliApp::run(&argv)
}

fn positionals() -> Vec<String> {
    std::env::args().skip(1).collect::<Vec<_>>()
}
