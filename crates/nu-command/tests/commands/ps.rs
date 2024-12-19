#[cfg(all(feature = "selinux", target_os = "linux"))]
#[test]
fn returns_correct_security_context() {
    use nu_test_support::nu_with_std;

    let input = "
        use std assert
        ^ps -o pid=,label= | lines | each { str trim | split column ' ' 'pid' 'procps_scontext' } | flatten \
        | join (ps -Z | each { default '-' security_context }) pid \
        | each { |e| assert equal $e.security_context $e.procps_scontext $'For process ($e.pid) expected ($e.procps_scontext), got ($e.security_context)' }
    ";
    assert_eq!(nu_with_std!(input).err, "");
}
