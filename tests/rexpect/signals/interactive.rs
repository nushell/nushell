use std::{thread::sleep, time::Duration};

use crate::rexpect::{nu_binary, spawn_nu_repl, NuReplExt};

use rexpect::{
    error::Error,
    process::{signal::Signal, wait::WaitStatus},
    spawn_bash,
};

#[test]
fn can_be_backgrounded_in_bash() -> Result<(), Error> {
    let mut p = spawn_bash(Some(1000))?;

    p.send_line(&format!("{} -n &", nu_binary()))?;
    p.wait_for_prompt()?;

    p.send_line("jobs")?;
    p.exp_string("[1]+  Stopped")?;

    p.send_line("exit")?;
    Ok(())
}

#[test]
fn internal_ctrl_c() -> Result<(), Error> {
    let mut p = spawn_nu_repl(Some(1000))?;

    p.send_nu_line("sleep 5sec")?;
    sleep(Duration::from_millis(500));
    p.send_control('c')?;
    p.exp_string("Operation interrupted by user")?;
    p.handle_prompt()?;

    p.send_nu_line("$env.LAST_EXIT_CODE")?;
    p.exp_string("1")?;
    p.handle_prompt()?;

    p.exit()
}

#[test]
fn external_ctrl_c() -> Result<(), Error> {
    let mut p = spawn_nu_repl(Some(1000))?;

    p.send_nu_line("bash -c read")?;
    sleep(Duration::from_millis(500));
    p.send_control('c')?;
    p.handle_prompt()?;

    // TODO: exp exit code > 0 ?
    // currently exit code is -1 for some reason

    p.exit()
}

#[test]
fn ctrlc_protection() -> Result<(), Error> {
    let mut p = spawn_nu_repl(Some(1000))?;

    sleep(Duration::from_millis(250));
    p.process.signal(Signal::SIGINT)?;
    sleep(Duration::from_millis(250));
    assert_eq!(p.process.status(), Some(WaitStatus::StillAlive));

    p.send_control('c')?;
    p.handle_prompt()?;

    p.exit()
}

#[test]
fn sigquit_protection() -> Result<(), Error> {
    let mut p = spawn_nu_repl(Some(1000))?;

    sleep(Duration::from_millis(250));
    p.process.signal(Signal::SIGQUIT)?;
    sleep(Duration::from_millis(250));
    assert_eq!(p.process.status(), Some(WaitStatus::StillAlive));

    p.exit()
}

#[test]
#[ignore] // currently fails, issue #7154
fn par_each_ctrl_c() -> Result<(), Error> {
    let mut p = spawn_nu_repl(Some(3000))?;

    const N: usize = 3;
    const MSG: &str = "sleeping";

    p.send_nu_line(&format!(
        "1..{N} | par-each {{ {} -c 'print -n {MSG}; sleep 5sec' }} | to nuon",
        nu_binary()
    ))?;
    // Sending ctrl-c too early triggers the internal ctrl-c handler? which will give no output.
    // We need to wait for the child nu processes to become the foreground process group
    // in order for the ctrl-c signal to be passed to them.
    sleep(Duration::from_millis(500));
    p.send_control('c')?;
    p.exp_string(&format!("[{}]", [MSG; N].join(", ")))?;
    p.handle_prompt()?;

    p.exit()
}
