// all tests in here are marked as serial in CI to reduce load while testing
// this improves the stability of these tests

use std::time::Duration;
use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
#[cfg_attr(ci, serial)]
fn job_send_root_job_works() -> Result {
    let code = "
        job spawn { 'beep' | job send 0 }
        job recv --timeout 10sec
    ";

    test().run(code).expect_value_eq("beep")
}

#[test]
#[cfg_attr(ci, serial)]
fn job_send_background_job_works() -> Result {
    let code = "
        let job = job spawn { job recv | job send 0 }
        'boop' | job send $job
        job recv --timeout 10sec
    ";

    test().run(code).expect_value_eq("boop")
}

#[test]
#[cfg_attr(ci, serial)]
fn job_send_to_self_works() -> Result {
    let code = r#"
        "meep" | job send 0
        job recv
    "#;

    test().run(code).expect_value_eq("meep")
}

#[test]
#[cfg_attr(ci, serial)]
fn job_send_to_self_from_background_works() -> Result {
    let code = "
        job spawn {
            'beep' | job send (job id)
            job recv | job send 0
        }

        job recv --timeout 10sec
    ";

    test().run(code).expect_value_eq("beep")
}

#[test]
#[cfg_attr(ci, serial)]
fn job_id_of_root_job_is_zero() -> Result {
    test().run("job id").expect_value_eq(0)
}

#[test]
#[cfg_attr(ci, serial)]
fn job_id_of_background_jobs_works() -> Result {
    let code = "
        let job1 = job spawn { job id | job send 0 }
        let id1 = job recv --timeout 5sec

        let job2 = job spawn { job id | job send 0 }
        let id2 = job recv --timeout 5sec

        let job3 = job spawn { job id | job send 0 }
        let id3 = job recv --timeout 5sec

        [($job1 == $id1) ($job2 == $id2) ($job3 == $id3)]

    ";

    test().run(code).expect_value_eq([true, true, true])
}

#[test]
#[cfg_attr(ci, serial)]
fn untagged_job_recv_accepts_tagged_messages() -> Result {
    let code = r#"
        job spawn { "boop" | job send 0 --tag 123 }
        job recv --timeout 10sec
    "#;

    test().run(code).expect_value_eq("boop")
}

#[test]
#[cfg_attr(ci, serial)]
fn tagged_job_recv_filters_untagged_messages() -> Result {
    let code = r#"
        job spawn { "boop" | job send 0 }
        job recv --tag 123 --timeout 1sec
    "#;

    let err = test().run(code).expect_shell_error()?;

    assert_contains("requested time interval", err.to_string());
    Ok(())
}

#[test]
#[cfg_attr(ci, serial)]
fn tagged_job_recv_filters_badly_tagged_messages() -> Result {
    let code = r#"
        job spawn { "boop" | job send 0 --tag 321 }
        job recv  --tag 123 --timeout 1sec
    "#;

    let err = test().run(code).expect_shell_error()?;

    assert_contains("requested time interval", err.to_string());
    Ok(())
}

#[test]
#[cfg_attr(ci, serial)]
fn tagged_job_recv_accepts_properly_tagged_messages() -> Result {
    let code = r#"
        job spawn { "boop" | job send 0 --tag 123 }
        job recv --tag 123 --timeout 5sec
    "#;

    test().run(code).expect_value_eq("boop")
}

#[test]
#[cfg_attr(ci, serial)]
fn filtered_messages_are_not_erased() -> Result {
    let code = r#"
        "msg1" | job send 0 --tag 123
        "msg2" | job send 0 --tag 456
        "msg3" | job send 0 --tag 789

        let first  = job recv --tag 789 --timeout 5sec
        let second = job recv --timeout 1sec
        let third  = job recv --timeout 1sec


        [($first) ($second) ($third)]
    "#;

    test().run(code).expect_value_eq(["msg3", "msg1", "msg2"])
}

#[test]
#[cfg_attr(ci, serial)]
fn job_recv_timeout_works() -> Result {
    let code = r#"
        job spawn {
            sleep 2sec
            "boop" | job send 0
        }

        job recv --timeout 1sec
    "#;

    let err = test().run(code).expect_shell_error()?;

    assert_contains("requested time interval", err.to_string());
    Ok(())
}

#[test]
#[cfg_attr(ci, serial)]
fn job_recv_timeout_zero_works() -> Result {
    let code = r#"
        "hi there" | job send 0
        job recv --timeout 0sec
    "#;

    test().run(code).expect_value_eq("hi there")
}

#[test]
#[cfg_attr(ci, serial)]
fn job_flush_clears_messages() -> Result {
    let code = r#"
        "SALE!!!" | job send 0
        "[HYPERLINK BLOCKED]" | job send 0

        job flush

        job recv --timeout 1sec
    "#;

    let err = test().run(code).expect_shell_error()?;

    assert_contains("requested time interval", err.to_string());
    Ok(())
}

#[test]
#[cfg_attr(ci, serial)]
fn job_flush_clears_filtered_messages() -> Result {
    let code = r#"
        "msg1" | job send 0 --tag 123
        "msg2" | job send 0 --tag 456
        "msg3" | job send 0 --tag 789

        job recv --tag 789 --timeout 1sec

        job flush

        job recv --timeout 1sec
    "#;

    let err = test().run(code).expect_shell_error()?;

    assert_contains("requested time interval", err.to_string());
    Ok(())
}

#[test]
#[cfg_attr(ci, serial)]
fn job_flush_with_tag() -> Result {
    let code = r#"
        "spam" | job send 0 --tag 404
        "not" | str reverse | job send 0 --tag 505
        "still alive" | job send 0 --tag 606
        "spam" | job send 0 --tag 404

        job recv --tag 505 --timeout 1sec

        job flush --tag 404

        job recv --timeout 1sec
    "#;

    test().run(code).expect_value_eq("still alive")
}

#[test]
#[cfg_attr(ci, serial)]
fn first_job_id_is_one() -> Result {
    test().run("job spawn {}").expect_value_eq(1)
}

#[test]
#[cfg_attr(ci, serial)]
fn job_list_adds_jobs_correctly() -> Result {
    let code = "
        let list0 = job list | get id;
        let job1 = job spawn { job recv };
        let list1 = job list | get id;
        let job2 = job spawn { job recv };
        let list2 = job list | get id;
        let job3 = job spawn { job recv };
        let list3 = job list | get id;
        [
            ($list0 == []),
            ($list1 == [$job1]),
            (($list2 | sort) == ([$job1, $job2] | sort)),
            (($list3 | sort) == ([$job1, $job2, $job3] | sort)),
        ]
    ";

    test().run(code).expect_value_eq([true, true, true, true])
}

#[test]
#[cfg_attr(ci, serial)]
fn jobs_get_removed_from_list_after_termination() -> Result {
    let code = r#"
        let job = job spawn { job recv };

        let list0 = job list | get id;

        "die!" | job send $job

        sleep 0.2sec

        let list1 = job list | get id;

        [($list0 == [$job]) ($list1 == [])]
    "#;

    test().run(code).expect_value_eq([true, true])
}

// TODO: find way to communicate between process in windows
// so these tests can fail less often
#[test]
#[cfg_attr(ci, serial)] // seems to fail less often with this
#[deps(NU)]
fn job_list_shows_pids() -> Result {
    let code = r#"
        let job1 = job spawn { nu -c "sleep 1sec" | nu -c "sleep 2sec" };
        sleep 500ms;
        let list0 = job list | where id == $job1 | first | get pids;
        sleep 1sec;
        let list1 = job list | where id == $job1 | first | get pids;
        [(($list0 | length) == 2), (($list1 | length) == 1), ($list1.0 in $list0)]
    "#;

    test().run(code).expect_value_eq([true, true, true])
}

#[test]
#[cfg_attr(ci, serial)]
fn killing_job_removes_it_from_table() -> Result {
    let code = "
        let job1 = job spawn { job recv }
        let job2 = job spawn { job recv }
        let job3 = job spawn { job recv }

        let list_before = job list | get id

        job kill $job1
        let list_after_kill_1 = job list | get id

        job kill $job2
        let list_after_kill_2 = job list | get id

        job kill $job3
        let list_after_kill_3 = job list | get id

        [
            (($list_before | sort) == ([$job1 $job2 $job3] | sort)),
            (($list_after_kill_1 | sort) == ([$job2 $job3] | sort)),
            (($list_after_kill_2 | sort) == ([$job3] | sort)),
            ($list_after_kill_3 == []),
        ]
    ";

    test().run(code).expect_value_eq([true, true, true, true])
}

// this test is unreliable on the macOS CI, but it worked fine for a couple months.
// still works on other operating systems.
#[rstest]
#[timeout(Duration::from_secs(3))]
#[nu_test_support::test]
#[cfg_attr(ci, serial)]
#[deps(NU)]
fn killing_job_kills_pids() -> Result {
    let code = r#"
        def wait-until [condition: closure, timeout: duration] {
            let deadline = (date now) + $timeout

            loop {
                if (do $condition) {
                    break
                }

                if (date now) >= $deadline {
                    error make { msg: "timed out waiting for condition" }
                }

                # Only prevents busy-spinning. Correctness doesn't depend on it.
                sleep 1ms
            }
        }

        # These remain blocked until the job is killed.
        let job1 = job spawn {
            nu -c "job recv" | nu -c "job recv"
        }

        # Wait until both processes have actually been registered.
        wait-until {
            (
                job list
                | where id == $job1
                | first
                | get pids
                | length
            ) == 2
        } 5sec

        let pids = (
            job list
            | where id == $job1
            | first
            | get pids
        )

        job kill $job1

        # Check the specific processes rather than all children of this Nu process.
        wait-until { ps | where pid in $pids | is-empty } 5sec
        true
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
#[cfg_attr(ci, serial)]
#[deps(NU)]
fn exiting_nushell_kills_jobs() -> Result {
    let code = r#"
        let result = nu -c "let job = job spawn { nu -c 'sleep 1sec' };
                sleep 100ms;
                let child_pid = job list | where id == $job | get pids | first;
                [$nu.pid $child_pid] | to nuon"

        let info = $result | from nuon
        let child_pid = $info.0
        let grandchild_pid = $info.1

        ps | where pid == $grandchild_pid | filter { $in.ppid in [$child_pid, 1] } | length
    "#;

    test().run(code).expect_value_eq(0)
}

#[cfg(unix)]
#[test]
#[cfg_attr(ci, serial)]
#[deps(NU)]
fn jobs_get_group_id_right() -> Result {
    let code = r#"
        let job1 = job spawn { nu -c "sleep 0.5sec" | nu -c "sleep 0.5sec"; }

        sleep 25ms

        let pids = job list | where id == $job1 | first | get pids

        let pid1 = $pids.0
        let pid2 = $pids.1

        let groups = ^ps -ax -o pid,pgid | from ssv -m 1 | update PID {|it| $it.PID | into int} | update PGID {|it| $it.PGID | into int}

        let my_group = $groups | where PID == $nu.pid | first | get PGID
        let group1 = $groups | where PID == $pid1 | first | get PGID
        let group2 = $groups | where PID == $pid2 | first | get PGID

        [($my_group != $group1) ($my_group != $group2) ($group1 == $group2)]
    "#;

    test().run(code).expect_value_eq([true, true, true])
}

#[test]
#[cfg_attr(ci, serial)]
#[deps(NU)]
fn job_extern_output_is_silent() -> Result {
    let result: CompleteResult = test().run_with_data(
        "nu -n -c $in | complete",
        r#" job spawn { nu -c "'hi'" }; sleep 1sec"#,
    )?;

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, "");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
#[cfg_attr(ci, serial)]
#[deps(NU)]
fn job_print_is_not_silent() -> Result {
    let result: CompleteResult = test().run_with_data(
        "nu -n -c $in | complete",
        r#" job spawn { print "hi" }; sleep 1sec"#,
    )?;

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "hi");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
#[cfg_attr(ci, serial)]
#[deps(NU)]
fn job_extern_into_value_is_not_silent() -> Result {
    let result: CompleteResult = test().run_with_data(
        "nu -n -c $in | complete",
        r#" job spawn { print (nu -c "'hi'") }; sleep 1sec"#,
    )?;

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "hi");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
#[cfg_attr(ci, serial)]
#[deps(NU)]
fn job_extern_into_pipe_is_not_silent() -> Result {
    let code = r#"
        job spawn {
            print (nu -c "10" | nu --stdin -c "($in | into int) + 1")
        }
        sleep 1sec
    "#;

    let result: CompleteResult = test().run_with_data("nu -n -c $in | complete", code)?;

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "11");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
#[cfg_attr(ci, serial)]
fn job_list_returns_no_description_when_job_is_undescribed() -> Result {
    let code = "
        job spawn { sleep 10sec }
        job spawn { sleep 10sec }
        job spawn { sleep 10sec }

        ('description' in (job list | columns))
    ";

    test().run(code).expect_value_eq(false)
}

#[test]
#[cfg_attr(ci, serial)]
fn job_list_returns_description_when_job_is_spawned_with_description() -> Result {
    let code = "
        job spawn { sleep 10sec } --description abc
        job list | where id == 1 | get description.0
    ";

    test().run(code).expect_value_eq("abc")
}

#[test]
#[cfg_attr(ci, serial)]
fn job_describe_modifies_descriptionless_job_desc() -> Result {
    let code = "
        job spawn { sleep 10sec }

        job describe 1 beep

        job list | where id == 1 | get description.0
    ";

    test().run(code).expect_value_eq("beep")
}

#[test]
#[cfg_attr(ci, serial)]
fn job_describe_modifies_described_job_description() -> Result {
    let code = "
        job spawn { sleep 10sec } --description abc

        job describe 1 beep

        job list | where id == 1 | get description.0
    ";

    test().run(code).expect_value_eq("beep")
}
