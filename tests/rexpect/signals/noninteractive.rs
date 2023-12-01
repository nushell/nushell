use std::{thread::sleep, time::Duration};

use crate::rexpect::{nu_binary, spawn_nu};

use rexpect::{
    error::Error,
    process::{signal::Signal, wait::WaitStatus},
    spawn_bash,
};

#[test]
fn can_be_backgrounded_in_bash() -> Result<(), Error> {
    let mut p = spawn_bash(Some(1000))?;

    p.send_line(&format!("{} -n -c 'sleep 100ms' &", nu_binary()))?;
    p.wait_for_prompt()?;

    sleep(Duration::from_millis(500));

    p.send_line("jobs")?;
    p.exp_string("[1]+  Done")?;

    p.send_line("exit")?;
    Ok(())
}

#[test]
#[ignore] // currently fails
fn ctrlc_internal() -> Result<(), Error> {
    let mut p = spawn_nu("sleep 1sec")?;
    sleep(Duration::from_millis(500));
    p.signal(Signal::SIGINT)?;
    let status = p.wait()?;
    assert!(
        matches!(status, WaitStatus::Signaled(_, Signal::SIGINT, _)),
        "process was not killed by SIGINT: {status:?}",
    );
    Ok(())
}

#[test]
#[ignore] // currently fails
fn sigquit_internal() -> Result<(), Error> {
    let mut p = spawn_nu("sleep 1sec")?;
    sleep(Duration::from_millis(500));
    p.signal(Signal::SIGQUIT)?;
    let status = p.wait()?;
    assert!(
        matches!(status, WaitStatus::Signaled(_, Signal::SIGQUIT, _)),
        "process was not killed by SIGQUIT: {status:?}",
    );
    Ok(())
}

#[test]
#[ignore] // currently fails
fn ctrlc_external() -> Result<(), Error> {
    let mut p = spawn_nu("bash -c read")?;
    sleep(Duration::from_millis(500));
    p.signal(Signal::SIGINT)?;
    let status = p.wait()?;
    assert!(
        matches!(status, WaitStatus::Signaled(_, Signal::SIGINT, _)),
        "process was not killed by SIGINT: {status:?}",
    );
    Ok(())
}

#[test]
#[ignore] // currently fails
fn sigquit_external() -> Result<(), Error> {
    let mut p = spawn_nu("bash -c read")?;
    sleep(Duration::from_millis(500));
    p.signal(Signal::SIGQUIT)?;
    let status = p.wait()?;
    assert!(
        matches!(status, WaitStatus::Signaled(_, Signal::SIGQUIT, _)),
        "process was not killed by SIGQUIT: {status:?}",
    );
    Ok(())
}
