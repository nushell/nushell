use std::path::Path;

use itertools::Itertools;
use nu_test_support::{fs::Stub, nu, playground::Playground};

#[test]
fn self_path_const() {
    Playground::setup("path_self_const", |dirs, sandbox| {
        sandbox
            .within("scripts")
            .with_files(&[Stub::FileWithContentToBeTrimmed(
                "foo.nu",
                r#"
                    export const paths = {
                        self: (path self),
                        dir: (path self .),
                        sibling: (path self sibling),
                        parent_dir: (path self ..),
                        cousin: (path self ../cousin),
                    }
                "#,
            )]);

        let actual = nu!(cwd: dirs.test(), r#"use scripts/foo.nu; $foo.paths | values | str join (char nul)"#);
        let (self_, dir, sibling, parent_dir, cousin) = actual
            .out
            .split("\0")
            .collect_tuple()
            .expect("should have 5 NUL separated paths");

        let mut pathbuf = dirs.test().to_path_buf();

        pathbuf.push("scripts");
        assert_eq!(pathbuf, Path::new(dir));

        pathbuf.push("foo.nu");
        assert_eq!(pathbuf, Path::new(self_));

        pathbuf.pop();
        pathbuf.push("sibling");
        assert_eq!(pathbuf, Path::new(sibling));

        pathbuf.pop();
        pathbuf.pop();
        assert_eq!(pathbuf, Path::new(parent_dir));

        pathbuf.push("cousin");
        assert_eq!(pathbuf, Path::new(cousin));
    })
}

#[test]
fn self_path_runtime() {
    let actual = nu!("path self");
    assert!(!actual.status.success());
    assert!(actual.err.contains("can only run during parse-time"));
}

#[test]
fn self_path_repl() {
    let actual = nu!("const foo = path self; $foo");
    assert!(!actual.status.success());
    assert!(actual.err.contains("nu::shell::io::not_found"));
}
