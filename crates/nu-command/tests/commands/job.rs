use nu_test_support::{nu, playground::Playground};

#[test]
fn jobs_do_run() {
    Playground::setup("job_test_1", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let actual = nu!(
            cwd: dirs.root(),
            r#"
            rm -f a.txt;
            job spawn { sleep 200ms; 'a' | save a.txt };
            let before = 'a.txt' | path exists;
            sleep 400ms;
            let after = 'a.txt' | path exists;
            [$before, $after] | to nuon"#
        );
        assert_eq!(actual.out, "[false, true]");
    })
}

#[test]
fn first_job_id_is_one() {
    let actual = nu!(r#"job spawn {} | to nuon"#);

    assert_eq!(actual.out, "1");
}

#[test]
fn job_list_adds_jobs_correctly() {
    let actual = nu!(format!(
        r#"
            let list0 = job list | get id;
            let job1 = job spawn {{ sleep 20ms }};
            let list1 = job list | get id;
            let job2 = job spawn {{ sleep 20ms }};
            let list2 = job list | get id;
            let job3 = job spawn {{ sleep 20ms }};
            let list3 = job list | get id;
            [({}), ({}), ({}), ({})] | to nuon
            "#,
        "$list0 == []",
        "$list1 == [$job1]",
        "($list2 | sort) == ([$job1, $job2] | sort)",
        "($list3 | sort) == ([$job1, $job2, $job3] | sort)"
    ));

    assert_eq!(actual.out, "[true, true, true, true]");
}

#[test]
fn jobs_get_removed_from_list_after_termination() {
    let actual = nu!(format!(
        r#"
            let job = job spawn {{ sleep 0.5sec }};

            let list0 = job list | get id;

            sleep 1sec

            let list1 = job list | get id;

            [({}) ({})] | to nuon
            "#,
        "$list0 == [$job]", "$list1 == []",
    ));

    assert_eq!(actual.out, "[true, true]");
}

#[test]
fn job_list_shows_pids() {
    let actual = nu!(format!(
        r#"
            let job1 = job spawn {{ nu -c "sleep 1sec" | nu -c "sleep 2sec" }};
            sleep 500ms;
            let list0 = job list | where id == $job1 | first | get pids;
            sleep 1sec;
            let list1 = job list | where id == $job1 | first | get pids;
            [({}), ({}), ({})] | to nuon
            "#,
        "($list0 | length) == 2", "($list1 | length) == 1", "$list1.0 in $list0",
    ));

    assert_eq!(actual.out, "[true, true, true]");
}

#[test]
fn killing_job_removes_it_from_table() {
    let actual = nu!(format!(
        r#"
            let job1 = job spawn {{ sleep 100ms }}
            let job2 = job spawn {{ sleep 100ms }}
            let job3 = job spawn {{ sleep 100ms }}

            let list_before = job list | get id

            job kill $job1
            let list_after_kill_1 = job list | get id

            job kill $job2
            let list_after_kill_2 = job list | get id

            job kill $job3
            let list_after_kill_3 = job list | get id
            
            [({}) ({}) ({}) ({})] | to nuon
            "#,
        "($list_before | sort) == ([$job1 $job2 $job3] | sort)",
        "($list_after_kill_1 | sort) == ([$job2 $job3] | sort)",
        "($list_after_kill_2 | sort) == ([$job3] | sort)",
        "$list_after_kill_3 == []",
    ));

    assert_eq!(actual.out, "[true, true, true, true]");
}

#[test]
fn killing_job_kills_pids() {
    let actual = nu!(format!(
        r#"
            let job1 = job spawn {{ nu -c "sleep 1sec" | nu -c "sleep 1sec" }}

            sleep 25ms

            let pids = job list | where id == $job1 | get pids

            let child_pids_before = ps | where ppid == $nu.pid

            job kill $job1
            
            sleep 25ms

            let child_pids_after = ps | where ppid == $nu.pid

            [({}) ({})] | to nuon
            "#,
        "($child_pids_before | length) == 2", "$child_pids_after == []",
    ));

    assert_eq!(actual.out, "[true, true]");
}

#[test]
fn exiting_nushell_kills_jobs() {
    let actual = nu!(r#"
            let result = nu -c "let job = job spawn { nu -c 'sleep 1sec' };
                   sleep 100ms;
                   let child_pid = job list | where id == $job | get pids | first;
                   [$nu.pid $child_pid] | to nuon"

            let info = $result | from nuon
            let child_pid = $info.0
            let grandchild_pid = $info.1

            ps | where pid == $grandchild_pid | filter { $in.ppid in [$child_pid, 1] } | length | to nuon
            "#);
    assert_eq!(actual.out, "0");
}

#[cfg(unix)]
#[test]
fn jobs_get_group_id_right() {
    let actual = nu!(r#"
            let job1 = job spawn { nu -c "sleep 0.5sec" | nu -c "sleep 0.5sec"; }

            sleep 25ms

            let pids = job list | where id == $job1 | first | get pids
            
            let pid1 = $pids.0
            let pid2 = $pids.1

            let groups = ^ps -ax -o pid,pgid | from ssv -m 1 | update PID {|it| $it.PID | into int} | update PGID {|it| $it.PGID | into int}

            let my_group = $groups | where PID == $nu.pid | first | get PGID
            let group1 = $groups | where PID == $pid1 | first | get PGID
            let group2 = $groups | where PID == $pid2 | first | get PGID

            [($my_group != $group1) ($my_group != $group2) ($group1 == $group2)] | to nuon
            "#,);

    assert_eq!(actual.out, "[true, true, true]");
}

#[test]
fn job_extern_output_is_silent() {
    let actual = nu!(r#" job spawn { nu -c "'hi'" }; sleep 1sec"#);
    assert_eq!(actual.out, "");
    assert_eq!(actual.err, "");
}

#[test]
fn job_print_is_not_silent() {
    let actual = nu!(r#" job spawn { print "hi" }; sleep 1sec"#);
    assert_eq!(actual.out, "hi");
    assert_eq!(actual.err, "");
}

#[test]
fn job_extern_into_value_is_not_silent() {
    let actual = nu!(r#" job spawn { print (nu -c "'hi'") }; sleep 1sec"#);
    assert_eq!(actual.out, "hi");
    assert_eq!(actual.err, "");
}

#[test]
fn job_extern_into_pipe_is_not_silent() {
    let actual = nu!(r#"
        job spawn { 
            print (nu -c "10" | nu --stdin -c "($in | into int) + 1")
        }
        sleep 1sec"#);

    assert_eq!(actual.out, "11");
    assert_eq!(actual.err, "");
}
