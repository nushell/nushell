use super::{nu_binary, spawn_nu, NuReplExt};

use rexpect::{error::Error, spawn_bash};

#[test]
fn can_be_backgrounded_in_bash() -> Result<(), Error> {
    let mut p = spawn_bash(Some(1000))?;

    p.send_line(&format!("{} -n &", nu_binary()))?;
    p.wait_for_prompt()?;

    p.send_line("jobs")?;
    p.exp_string("[1]+  Stopped")?;
    p.wait_for_prompt()?;

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

    p.sendline(r#"1..3 | par-each { python -c 'import time; print("sleeping"); time.sleep(5)' }; print done"#)?;
    for _ in 1..=3 {
        p.exp_string("sleeping")?;
    }

    p.send_control('c')?;
    p.exp_string("done")?;

    p.exit()
}
