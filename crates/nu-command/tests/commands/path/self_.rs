use std::path::Path;

use itertools::Itertools;
use nu_test_support::{fs::Stub, prelude::*};

#[test]
fn self_path_const() -> Result {
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

        let code = r#"use scripts/foo.nu; $foo.paths | values | str join (char nul)"#;
        let outcome: String = test().cwd(dirs.test()).run(code)?;
        let (self_, dir, sibling, parent_dir, cousin) = outcome
            .split('\0')
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
        Ok(())
    })
}

#[test]
fn self_path_runtime() -> Result {
    let err = test().run("path self").expect_shell_error()?;
    assert_contains("can only run during parse-time", err.to_string());
    Ok(())
}

#[test]
fn self_path_repl() -> Result {
    let code = "const foo = path self; $foo";
    let err = test().run(code).expect_parse_error()?;
    assert_contains("nu::shell::io::file_not_found", err.to_string());
    Ok(())
}
