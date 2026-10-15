use nu_cli::update_prompt;
use nu_test_support::prelude::*;

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
