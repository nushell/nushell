use nu_test_support::nu;

#[test]
fn for_doesnt_auto_print_in_each_iteration() {
    let actual = nu!("
        for i in 1..2 {
            $i
        }");
    // Make sure we don't see any of these values in the output
    // As we do not auto-print loops anymore
    assert!(!actual.out.contains('1'));
}

#[test]
fn for_break_on_external_failed() {
    let actual = nu!("
        for i in 1..2 {
            print 1;
            nu --testbin fail
        }");
    // Note: nu! macro auto replace "\n" and "\r\n" with ""
    // so our output will be `1`
    assert_eq!(actual.out, "1");
}

#[test]
fn failed_for_should_break_running() {
    let actual = nu!("
        for i in 1..2 {
            nu --testbin fail
        }
        print 3");
    assert!(!actual.out.contains('3'));

    let actual = nu!("
        let x = [1 2]
        for i in $x {
            nu --testbin fail
        }
        print 3");
    assert!(!actual.out.contains('3'));
}

#[test]
fn for_loops_dont_collect_source() {
    let actual = nu!("
        for i in (seq 1 10 | each { print -n $in; $in}) {
            print -n $i
            if $i >= 5 { break }
        }
    ");
    assert_eq!(actual.out, "1122334455");
}
