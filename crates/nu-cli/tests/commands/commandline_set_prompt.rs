use nu_cli::update_prompt;
use nu_protocol::engine::EngineState;
use nu_test_support::prelude::*;
use nu_utils::time::Instant;
use std::{sync::Arc, time::Duration};

// Each test opens the window the REPL opens before its hooks run (`PromptState::begin_update`),
// runs what a hook would, then installs the prompt baseline the way the REPL does. Jobs report
// back with `job send 0`, so the tests wait for them without sleeping.

#[test]
fn push_from_a_job_started_for_the_prompt_survives_the_baseline() -> Result {
    let mut tester = test().env("PROMPT_COMMAND", "baseline> ");
    let first_job = tester
        .engine_state
        .jobs
        .lock()
        .expect("jobs lock")
        .next_id();
    tester.engine_state.prompt_state.begin_update(first_job);

    let () = tester.run(
        "job spawn { commandline set-prompt 'pushed> '; 'done' | job send 0 }; job recv --timeout 1min | ignore",
    )?;
    let config = tester.engine_state.get_config().clone();
    update_prompt(&config, &tester.engine_state, &mut tester.stack);

    let left = tester.engine_state.prompt_state.contents().left;
    assert_eq!(left.as_deref(), Some("pushed> "));
    Ok(())
}

#[test]
fn push_from_a_job_spawned_by_an_earlier_job_does_not_survive_the_baseline() -> Result {
    let mut tester = test().env("PROMPT_COMMAND", "baseline> ");
    // A job left over from an earlier prompt: it spawns the job that pushes once told to.
    let earlier: i64 = tester.run(
        "job spawn { job recv | ignore; job spawn { commandline set-prompt 'stale> '; 'done' | job send 0 } }",
    )?;
    let first_job = tester
        .engine_state
        .jobs
        .lock()
        .expect("jobs lock")
        .next_id();
    tester.engine_state.prompt_state.begin_update(first_job);

    let () = tester.run_with_data(
        "let earlier = $in; 'go' | job send $earlier; job recv --timeout 1min | ignore",
        earlier,
    )?;
    let config = tester.engine_state.get_config().clone();
    update_prompt(&config, &tester.engine_state, &mut tester.stack);

    let left = tester.engine_state.prompt_state.contents().left;
    assert_eq!(left.as_deref(), Some("baseline> "));
    Ok(())
}

// While the REPL's repainter is installed, reedline polls for input instead of blocking, so the
// tests below check that the last job thread to end drops it. Each installs a stand-in that
// holds a clone of the returned `Arc`: a strong count of 2 means it's installed, 1 means dropped.
// They poll with `wait_until` instead of `job recv`: a job thread leaves the table only after
// the last thing it could send.

fn install_repainter(engine_state: &EngineState) -> Arc<()> {
    let alive = Arc::new(());
    let held = Arc::clone(&alive);
    engine_state
        .prompt_state
        .set_repainter(Some(Arc::new(move || {
            let _ = &held;
        })));
    alive
}

/// Wait for something a job thread does after its closure returns, so after anything it sends.
fn wait_until(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn job_count(engine_state: &EngineState) -> usize {
    engine_state.jobs.lock().expect("jobs lock").iter().count()
}

#[test]
fn last_job_to_end_drops_the_repainter() -> Result {
    let mut tester = test();
    let repainter = install_repainter(&tester.engine_state);

    let _: i64 = tester.run("job spawn {}")?;
    wait_until("the job ends", || job_count(&tester.engine_state) == 0);

    assert_eq!(Arc::strong_count(&repainter), 1);
    Ok(())
}

#[test]
fn repainter_stays_while_another_job_runs() -> Result {
    let mut tester = test();
    let repainter = install_repainter(&tester.engine_state);

    let waiting: i64 = tester.run("job spawn { job recv | ignore }")?;
    let _: i64 = tester.run("job spawn {}")?;
    wait_until("the second job ends", || {
        job_count(&tester.engine_state) == 1
    });
    assert_eq!(Arc::strong_count(&repainter), 2);

    let () = tester.run_with_data("let waiting = $in; 'go' | job send $waiting", waiting)?;
    wait_until("the first job ends", || {
        job_count(&tester.engine_state) == 0
    });
    assert_eq!(Arc::strong_count(&repainter), 1);
    Ok(())
}

#[test]
fn killed_job_drops_the_repainter_when_its_thread_ends() -> Result {
    let mut tester = test();
    let repainter = install_repainter(&tester.engine_state);

    // `job kill` takes the job out of the table before its thread notices and ends.
    let () = tester.run("let id = job spawn { job recv | ignore }; job kill $id")?;
    wait_until("the killed job's thread drops the repainter", || {
        Arc::strong_count(&repainter) == 1
    });
    Ok(())
}
