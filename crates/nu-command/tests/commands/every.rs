use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::prelude::*;

#[test]
fn gets_all_rows_by_every_zero() -> Result {
    Playground::setup("every_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls | get name | every 0")
            .expect_value_eq(["amigos.txt", "arepas.clu", "los.txt", "tres.txt"])
    })
}

#[test]
fn gets_no_rows_by_every_skip_zero() -> Result {
    Playground::setup("every_test_2", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls | get name | every 0 --skip")
            .expect_value_eq(Vec::<String>::new())
    })
}

#[test]
fn gets_all_rows_by_every_one() -> Result {
    Playground::setup("every_test_3", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls | get name | every 1")
            .expect_value_eq(["amigos.txt", "arepas.clu", "los.txt", "tres.txt"])
    })
}

#[test]
fn gets_no_rows_by_every_skip_one() -> Result {
    Playground::setup("every_test_4", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls | get name | every 1 --skip")
            .expect_value_eq(Vec::<String>::new())
    })
}

#[test]
fn gets_first_row_by_every_too_much() -> Result {
    Playground::setup("every_test_5", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls | get name | every 999")
            .expect_value_eq(["amigos.txt"])
    })
}

#[test]
fn gets_all_rows_except_first_by_every_skip_too_much() -> Result {
    Playground::setup("every_test_6", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls | get name | every 999 --skip")
            .expect_value_eq(["arepas.clu", "los.txt", "tres.txt"])
    })
}

#[test]
fn gets_every_third_row() -> Result {
    Playground::setup("every_test_7", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("quatro.txt"),
            EmptyFile("tres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls | get name | every 3")
            .expect_value_eq(["amigos.txt", "quatro.txt"])
    })
}

#[test]
fn skips_every_third_row() -> Result {
    Playground::setup("every_test_8", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("quatro.txt"),
            EmptyFile("tres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls | get name | every 3 --skip")
            .expect_value_eq(["arepas.clu", "los.txt", "tres.txt"])
    })
}
