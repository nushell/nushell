use nu_test_support::prelude::*;

#[test]
fn gets_all_rows_by_every_zero(playground: Playground) -> Result {
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | get name | every 0")
        .expect_value_eq(["amigos.txt", "arepas.clu", "los.txt", "tres.txt"])
}

#[test]
fn gets_no_rows_by_every_skip_zero(playground: Playground) -> Result {
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | get name | every 0 --skip")
        .expect_value_eq(Vec::<String>::new())
}

#[test]
fn gets_all_rows_by_every_one(playground: Playground) -> Result {
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | get name | every 1")
        .expect_value_eq(["amigos.txt", "arepas.clu", "los.txt", "tres.txt"])
}

#[test]
fn gets_no_rows_by_every_skip_one(playground: Playground) -> Result {
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | get name | every 1 --skip")
        .expect_value_eq(Vec::<String>::new())
}

#[test]
fn gets_first_row_by_every_too_much(playground: Playground) -> Result {
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | get name | every 999")
        .expect_value_eq(["amigos.txt"])
}

#[test]
fn gets_all_rows_except_first_by_every_skip_too_much(playground: Playground) -> Result {
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file("los.txt")?;
    playground.empty_file("tres.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | get name | every 999 --skip")
        .expect_value_eq(["arepas.clu", "los.txt", "tres.txt"])
}

#[test]
fn gets_every_third_row(playground: Playground) -> Result {
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file("los.txt")?;
    playground.empty_file("quatro.txt")?;
    playground.empty_file("tres.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | get name | every 3")
        .expect_value_eq(["amigos.txt", "quatro.txt"])
}

#[test]
fn skips_every_third_row(playground: Playground) -> Result {
    playground.empty_file("amigos.txt")?;
    playground.empty_file("arepas.clu")?;
    playground.empty_file("los.txt")?;
    playground.empty_file("quatro.txt")?;
    playground.empty_file("tres.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | get name | every 3 --skip")
        .expect_value_eq(["arepas.clu", "los.txt", "tres.txt"])
}
