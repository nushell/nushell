use nu_test_support::nu;
use nu_test_support::playground::Playground;
#[test]
fn test_uname_all() {
    Playground::setup("uname_test_1", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "uname -a"
        );
        assert!(actual.status.success())
    })
}

#[test]
fn test_uname_kernel_name() {
    Playground::setup("uname_test_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "uname -s"
        );
        assert!(actual.status.success())
    })
}

#[test]
fn test_uname_nodename() {
    Playground::setup("uname_test_3", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "uname -n"
        );
        assert!(actual.status.success())
    })
}

#[test]
fn test_uname_kernel_version() {
    Playground::setup("uname_test_4", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "uname -v"
        );
        assert!(actual.status.success())
    })
}
#[test]
fn test_uname_machine() {
    Playground::setup("uname_test_5", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "uname -m"
        );
        assert!(actual.status.success())
    })
}
#[test]
fn test_uname_operating_system() {
    Playground::setup("uname_test_6", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "uname --operating-system"
        );
        #[cfg(all(target_os = "linux", any(target_env = "gnu", target_env = "")))]

        assert!(actual.out.contains("GNU/Linux"));
        #[cfg(target_vendor = "apple")]
        assert!(actual.out.contains("Darwin"));
        #[cfg(target_os = "windows")]
        assert!(actual.out.starts_with("MS/Windows"));
    })
}
