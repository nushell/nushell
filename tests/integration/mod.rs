use nu_test_support::prelude::*;

mod cli;
mod posix_end_of_options;
mod shell_integration;

#[test]
fn multiword_commands_have_their_parent_commands() -> Result {
    let code = "
        scope commands
        | where type == built-in and name like ' '
        | where ($it.name | split row ' ' | first) not-in (
            scope commands
            | where type in [keyword built-in]
            | get name
        )
        | get name
    ";
    let out: Vec<String> = test().run(code)?;

    match out.as_slice() {
        [] => Ok(()),
        cmds => {
            panic!("These multiword commands are missing their dummy parent commands: {cmds:?}")
        }
    }
}
