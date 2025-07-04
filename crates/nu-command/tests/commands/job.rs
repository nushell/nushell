use nu_test_support::nu;

#[test]
fn job_send_root_job_works() {
    let actual = nu!(r#"
        job spawn { 'beep' | job send 0 }
        job recv --timeout 10sec"#);

    assert_eq!(actual.out, "beep");
}

#[test]
fn job_send_background_job_works() {
    let actual = nu!(r#"
        let job = job spawn { job recv | job send 0 }
        'boop' | job send $job
        job recv --timeout 10sec"#);

    assert_eq!(actual.out, "boop");
}

#[test]
fn job_send_to_self_works() {
    let actual = nu!(r#"
        "meep" | job send 0
        job recv"#);

    assert_eq!(actual.out, "meep");
}

#[test]
fn job_send_to_self_from_background_works() {
    let actual = nu!(r#"
        job spawn {
            'beep' | job send (job id)
            job recv | job send 0
        }

        job recv --timeout 10sec"#);

    assert_eq!(actual.out, "beep");
}

#[test]
fn job_id_of_root_job_is_zero() {
    let actual = nu!(r#"job id"#);

    assert_eq!(actual.out, "0");
}

#[test]
fn job_id_of_background_jobs_works() {
    let actual = nu!(r#"
        let job1 = job spawn { job id | job send 0 }
        let id1 = job recv --timeout 5sec

        let job2 = job spawn { job id | job send 0 }
        let id2 = job recv --timeout 5sec

        let job3 = job spawn { job id | job send 0 }
        let id3 = job recv --timeout 5sec 

        [($job1 == $id1) ($job2 == $id2) ($job3 == $id3)] | to nuon

        "#);

    assert_eq!(actual.out, "[true, true, true]");
}

#[test]
fn untagged_job_recv_accepts_tagged_messages() {
    let actual = nu!(r#"
        job spawn { "boop" | job send 0 --tag 123 }
        job recv --timeout 10sec
        "#);

    assert_eq!(actual.out, "boop");
}

#[test]
fn tagged_job_recv_filters_untagged_messages() {
    let actual = nu!(r#"
        job spawn { "boop" | job send 0 }
        job recv --tag 123 --timeout 1sec
        "#);

    assert_eq!(actual.out, "");
    assert!(actual.err.contains("timeout"));
}

#[test]
fn tagged_job_recv_filters_badly_tagged_messages() {
    let actual = nu!(r#"
        job spawn { "boop" | job send 0 --tag 321 }
        job recv  --tag 123 --timeout 1sec
        "#);

    assert_eq!(actual.out, "");
    assert!(actual.err.contains("timeout"));
}

#[test]
fn tagged_job_recv_accepts_properly_tagged_messages() {
    let actual = nu!(r#"
        job spawn { "boop" | job send 0 --tag 123 }
        job recv --tag 123 --timeout 5sec
        "#);

    assert_eq!(actual.out, "boop");
}

#[test]
fn filtered_messages_are_not_erased() {
    let actual = nu!(r#"
        "msg1" | job send 0 --tag 123
        "msg2" | job send 0 --tag 456
        "msg3" | job send 0 --tag 789

        let first  = job recv --tag 789 --timeout 5sec
        let second = job recv --timeout 1sec
        let third  = job recv --timeout 1sec
        

        [($first) ($second) ($third)] | to nuon
        "#);

    assert_eq!(actual.out, r#"["msg3", "msg1", "msg2"]"#);
}

#[test]
fn job_recv_timeout_works() {
    let actual = nu!(r#"
        job spawn { 
            sleep 2sec
            "boop" | job send 0
        }

        job recv --timeout 1sec
        "#);

    assert_eq!(actual.out, "");
    assert!(actual.err.contains("timeout"));
}

#[test]
fn job_recv_timeout_zero_works() {
    let actual = nu!(r#"
        "hi there" | job send 0
        job recv --timeout 0sec
        "#);

    assert_eq!(actual.out, "hi there");
}

#[test]
fn job_flush_clears_messages() {
    let actual = nu!(r#"
        "SALE!!!" | job send 0
        "[HYPERLINK BLOCKED]" | job send 0

        job flush

        job recv --timeout 1sec
        "#);

    assert_eq!(actual.out, "");
    assert!(actual.err.contains("timeout"));
}

#[test]
fn job_flush_clears_filtered_messages() {
    let actual = nu!(r#"
        "msg1" | job send 0 --tag 123
        "msg2" | job send 0 --tag 456
        "msg3" | job send 0 --tag 789

        job recv --tag 789 --timeout 1sec

        job flush

        job recv --timeout 1sec
        "#);

    assert_eq!(actual.out, "");
    assert!(actual.err.contains("timeout"));
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
            let job1 = job spawn {{ job recv }};
            let list1 = job list | get id;
            let job2 = job spawn {{ job recv }};
            let list2 = job list | get id;
            let job3 = job spawn {{ job recv }};
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
            let job = job spawn {{ job recv }};

            let list0 = job list | get id;

            "die!" | job send $job

            sleep 0.2sec

            let list1 = job list | get id;

            [({}) ({})] | to nuon
            "#,
        "$list0 == [$job]", "$list1 == []",
    ));

    assert_eq!(actual.out, "[true, true]");
}

// TODO: find way to communicate between process in windows
// so these tests can fail less often
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
            let job1 = job spawn {{ job recv }}
            let job2 = job spawn {{ job recv }}
            let job3 = job spawn {{ job recv }}

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

// this test is unreliable on the macOS CI, but it worked fine for a couple months.
// still works on other operating systems.
#[test]
#[cfg(not(target_os = "macos"))]
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

#[test]
fn job_list_returns_no_tag_when_job_is_untagged() {
    let actual = nu!(r#"
        job spawn { sleep 10sec }
        job spawn { sleep 10sec }
        job spawn { sleep 10sec }

        ('tag' in (job list | columns)) | to nuon"#);

    assert_eq!(actual.out, "false");
    assert_eq!(actual.err, "");
}

#[test]
fn job_list_returns_tag_when_job_is_spawned_with_tag() {
    let actual = nu!(r#"
        job spawn { sleep 10sec } --tag abc
        job list | where id == 1 | get tag.0
        "#);

    assert_eq!(actual.out, "abc");
    assert_eq!(actual.err, "");
}

#[test]
fn job_tag_modifies_untagged_job_tag() {
    let actual = nu!(r#"
        job spawn { sleep 10sec }

        job tag 1 beep
        
        job list | where id == 1 | get tag.0"#);

    assert_eq!(actual.out, "beep");
    assert_eq!(actual.err, "");
}

#[test]
fn job_tag_modifies_tagged_job_tag() {
    let actual = nu!(r#"
        job spawn { sleep 10sec } --tag abc

        job tag 1 beep

        job list | where id == 1 | get tag.0"#);

    assert_eq!(actual.out, "beep");
    assert_eq!(actual.err, "");
}
