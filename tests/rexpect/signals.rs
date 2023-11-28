use std::time::Duration;

use super::{nu_binary, spawn_nu, NuReplExt};

use rexpect::{error::Error, spawn_bash};

#[test]
fn can_be_backgrounded_in_bash() -> Result<(), Error> {
    let mut p = spawn_bash(Some(1000))?;

    p.send_line(&format!("{} -n &", nu_binary()))?;
    p.wait_for_prompt()?;

    p.send_line("jobs")?;
    p.exp_string("[1]+  Stopped")?;

    p.send_line("kill %1")?;
    p.wait_for_prompt()?;

    p.send_line("exit")?;
    Ok(())
}

#[test]
#[ignore] // currently fails, issue #7154
fn par_each_ctrl_c() -> Result<(), Error> {
    let mut p = spawn_nu(Some(3000))?;
    p.wait_for_prompt()?;

    const N: usize = 3;
    const MSG: &str = "sleeping";

    p.sendline(&format!(
        "1..{N} | par-each {{ {} -c 'print -n {MSG}; sleep 5sec' }} | to nuon",
        nu_binary()
    ))?;
    // Sending ctrl-c too early triggers the internal ctrl-c handler? which will give no output.
    // We need to wait for the child nu processes to become the foreground process group
    // in order for the ctrl-c signal to be passed to them.
    std::thread::sleep(Duration::from_millis(500));
    p.send_control('c')?;
    p.exp_string(&format!("[{}]", [MSG; N].join(", ")))?;

    p.exit()
}
