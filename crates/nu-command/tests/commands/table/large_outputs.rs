use indoc::indoc;
use nu_test_support::{fs::Stub::FileWithContent, prelude::*};

#[test]
fn table_pagging_row_offset_overlap() -> Result {
    let actual: String = test().run("0..1000 | table")?;
    let rows = (0..1000)
        .map(|i| format!("│{i:>4} │{i:>4} │"))
        .collect::<Vec<_>>()
        .join("\n");
    let expected = format!(
        "{}\n{}\n{}",
        "╭─────┬─────╮",
        rows,
        indoc! {"
            ╰─────┴─────╯
            ╭──────┬──────╮
            │ 1000 │ 1000 │
            ╰──────┴──────╯
        "}
    );
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn table_index_0() -> Result {
    let actual: String = test().run("[1 3 1 3 2 1 1] | table")?;
    assert_eq!(
        actual,
        indoc! {"
            ╭───┬───╮
            │ 0 │ 1 │
            │ 1 │ 3 │
            │ 2 │ 1 │
            │ 3 │ 3 │
            │ 4 │ 2 │
            │ 5 │ 1 │
            │ 6 │ 1 │
            ╰───┴───╯
        "}
    );
    Ok(())
}

#[test]
fn test_expand_big_0() -> Result {
    Playground::setup("test_expand_big_0", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
            [package]
            authors = ["The Nushell Project Developers"]
            default-run = "nu"
            description = "A new type of shell"
            documentation = "https://www.nushell.sh/book/"
            edition = "2024"
            exclude = ["images"]
            homepage = "https://www.nushell.sh"
            license = "MIT"
            name = "nu"
            repository = "https://github.com/nushell/nushell"
            rust-version = "1.60"
            version = "0.74.1"


            # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html


            [package.metadata.binstall]
            pkg-url = "{ repo }/releases/download/{ version }/{ name }-{ version }-{ target }.{ archive-format }"
            pkg-fmt = "tgz"


            [package.metadata.binstall.overrides.x86_64-pc-windows-msvc]
            pkg-fmt = "zip"


            [workspace]
            members = [
                "crates/nu-cli",
                "crates/nu-engine",
                "crates/nu-parser",
                "crates/nu-system",
                "crates/nu-command",
                "crates/nu-protocol",
                "crates/nu-plugin",
                "crates/nu_plugin_inc",
                "crates/nu_plugin_gstat",
                "crates/nu_plugin_example",
                "crates/nu_plugin_query",
                "crates/nu_plugin_custom_values",
                "crates/nu-utils",
            ]


            [dependencies]
            chrono = { version = "0.4.23", features = ["serde"] }
            crossterm = "0.24.0"
            ctrlc = "3.2.1"
            log = "0.4"
            miette = { version = "5.5.0", features = ["fancy-no-backtrace"] }
            nu-ansi-term = "0.46.0"
            nu-cli = { path = "./crates/nu-cli", version = "0.74.1" }
            nu-engine = { path = "./crates/nu-engine", version = "0.74.1" }
            reedline = { version = "0.14.0", features = ["bashisms", "sqlite"] }


            rayon = "1.6.1"
            is_executable = "1.0.1"
            simplelog = "0.12.0"
            time = "0.3.12"


            [target.'cfg(not(target_os = "windows"))'.dependencies]
            # Our dependencies don't use OpenSSL on Windows
            openssl = { version = "0.10.38", features = ["vendored"], optional = true }
            signal-hook = { version = "0.3.14", default-features = false }




            [target.'cfg(windows)'.build-dependencies]
            winres = "0.1"


            [target.'cfg(target_family = "unix")'.dependencies]
            nix = { version = "0.25", default-features = false, features = ["signal", "process", "fs", "term"] }
            atty = "0.2"


            [dev-dependencies]
            nu-test-support = { path = "./crates/nu-test-support", version = "0.74.1" }
            tempfile = "3.2.0"
            assert_cmd = "2.0.2"
            criterion = "0.4"
            pretty_assertions = "1.0.0"
            serial_test = "0.10.0"
            hamcrest2 = "0.3.0"
            rstest = { version = "0.15.0", default-features = false }
            itertools = "0.10.3"


            [features]
            plugin = [
                "nu-plugin",
                "nu-cli/plugin",
                "nu-parser/plugin",
                "nu-command/plugin",
                "nu-protocol/plugin",
                "nu-engine/plugin",
            ]
            # extra used to be more useful but now it's the same as default. Leaving it in for backcompat with existing build scripts
            extra = ["default"]
            default = ["plugin", "which-support", "trash-support", "sqlite"]
            stable = ["default"]
            wasi = []


            # Enable to statically link OpenSSL; otherwise the system version will be used. Not enabled by default because it takes a while to build
            static-link-openssl = ["dep:openssl"]


            # Stable (Default)
            which-support = ["nu-command/which-support"]
            trash-support = ["nu-command/trash-support"]


            # Main nu binary
            [[bin]]
            name = "nu"
            path = "src/main.rs"


            # To use a development version of a dependency please use a global override here
            # changing versions in each sub-crate of the workspace is tedious
            [patch.crates-io]
            reedline = { git = "https://github.com/nushell/reedline.git", branch = "main" }


            # Criterion benchmarking setup
            # Run all benchmarks with `cargo bench`
            # Run individual benchmarks like `cargo bench -- <regex>` e.g. `cargo bench -- parse`
            [[bench]]
            name = "benchmarks"
            harness = false
            "#,
        )]);
        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --width=80 --expand")?;
        let expected = indoc! {r#"
            ╭──────────────────┬───────────────────────────────────────────────────────────╮
            │                  │ ╭───────────────┬───────────────────────────────────────╮ │
            │ package          │ │               │ ╭───┬──────────────────────╮          │ │
            │                  │ │ authors       │ │ 0 │ The Nushell Project  │          │ │
            │                  │ │               │ │   │ Developers           │          │ │
            │                  │ │               │ ╰───┴──────────────────────╯          │ │
            │                  │ │ default-run   │ nu                                    │ │
            │                  │ │ description   │ A new type of shell                   │ │
            │                  │ │ documentation │ https://www.nushell.sh/book/          │ │
            │                  │ │ edition       │ 2024                                  │ │
            │                  │ │               │ ╭───┬────────╮                        │ │
            │                  │ │ exclude       │ │ 0 │ images │                        │ │
            │                  │ │               │ ╰───┴────────╯                        │ │
            │                  │ │ homepage      │ https://www.nushell.sh                │ │
            │                  │ │ license       │ MIT                                   │ │
            │                  │ │ name          │ nu                                    │ │
            │                  │ │ repository    │ https://github.com/nushell/nushell    │ │
            │                  │ │ rust-version  │ 1.60                                  │ │
            │                  │ │ version       │ 0.74.1                                │ │
            │                  │ │               │ ╭──────────┬────────────────────────╮ │ │
            │                  │ │ metadata      │ │          │ ╭───────────┬────────╮ │ │ │
            │                  │ │               │ │ binstall │ │ pkg-url   │ { repo │ │ │ │
            │                  │ │               │ │          │ │           │  }/rel │ │ │ │
            │                  │ │               │ │          │ │           │ eases/ │ │ │ │
            │                  │ │               │ │          │ │           │ downlo │ │ │ │
            │                  │ │               │ │          │ │           │ ad/{ v │ │ │ │
            │                  │ │               │ │          │ │           │ ersion │ │ │ │
            │                  │ │               │ │          │ │           │  }/{   │ │ │ │
            │                  │ │               │ │          │ │           │ name   │ │ │ │
            │                  │ │               │ │          │ │           │ }-{ ve │ │ │ │
            │                  │ │               │ │          │ │           │ rsion  │ │ │ │
            │                  │ │               │ │          │ │           │ }-{    │ │ │ │
            │                  │ │               │ │          │ │           │ target │ │ │ │
            │                  │ │               │ │          │ │           │  }.{ a │ │ │ │
            │                  │ │               │ │          │ │           │ rchive │ │ │ │
            │                  │ │               │ │          │ │           │ -forma │ │ │ │
            │                  │ │               │ │          │ │           │ t }    │ │ │ │
            │                  │ │               │ │          │ │ pkg-fmt   │ tgz    │ │ │ │
            │                  │ │               │ │          │ │ overrides │ {recor │ │ │ │
            │                  │ │               │ │          │ │           │ d 1    │ │ │ │
            │                  │ │               │ │          │ │           │ field} │ │ │ │
            │                  │ │               │ │          │ ╰───────────┴────────╯ │ │ │
            │                  │ │               │ ╰──────────┴────────────────────────╯ │ │
            │                  │ ╰───────────────┴───────────────────────────────────────╯ │
            │                  │ ╭───────────┬───────────────────────────────────────────╮ │
            │ workspace        │ │           │ ╭────┬────────────────────────────────╮   │ │
            │                  │ │ members   │ │  0 │ crates/nu-cli                  │   │ │
            │                  │ │           │ │  1 │ crates/nu-engine               │   │ │
            │                  │ │           │ │  2 │ crates/nu-parser               │   │ │
            │                  │ │           │ │  3 │ crates/nu-system               │   │ │
            │                  │ │           │ │  4 │ crates/nu-command              │   │ │
            │                  │ │           │ │  5 │ crates/nu-protocol             │   │ │
            │                  │ │           │ │  6 │ crates/nu-plugin               │   │ │
            │                  │ │           │ │  7 │ crates/nu_plugin_inc           │   │ │
            │                  │ │           │ │  8 │ crates/nu_plugin_gstat         │   │ │
            │                  │ │           │ │  9 │ crates/nu_plugin_example       │   │ │
            │                  │ │           │ │ 10 │ crates/nu_plugin_query         │   │ │
            │                  │ │           │ │ 11 │ crates/nu_plugin_custom_values │   │ │
            │                  │ │           │ │ 12 │ crates/nu-utils                │   │ │
            │                  │ │           │ ╰────┴────────────────────────────────╯   │ │
            │                  │ ╰───────────┴───────────────────────────────────────────╯ │
            │                  │ ╭───────────────┬───────────────────────────────────────╮ │
            │ dependencies     │ │               │ ╭──────────┬───────────────╮          │ │
            │                  │ │ chrono        │ │ version  │ 0.4.23        │          │ │
            │                  │ │               │ │          │ ╭───┬───────╮ │          │ │
            │                  │ │               │ │ features │ │ 0 │ serde │ │          │ │
            │                  │ │               │ │          │ ╰───┴───────╯ │          │ │
            │                  │ │               │ ╰──────────┴───────────────╯          │ │
            │                  │ │ crossterm     │ 0.24.0                                │ │
            │                  │ │ ctrlc         │ 3.2.1                                 │ │
            │                  │ │ log           │ 0.4                                   │ │
            │                  │ │               │ ╭──────────┬────────────────────────╮ │ │
            │                  │ │ miette        │ │ version  │ 5.5.0                  │ │ │
            │                  │ │               │ │          │ ╭───┬────────────────╮ │ │ │
            │                  │ │               │ │ features │ │ 0 │ fancy-no-backt │ │ │ │
            │                  │ │               │ │          │ │   │ race           │ │ │ │
            │                  │ │               │ │          │ ╰───┴────────────────╯ │ │ │
            │                  │ │               │ ╰──────────┴────────────────────────╯ │ │
            │                  │ │ nu-ansi-term  │ 0.46.0                                │ │
            │                  │ │               │ ╭─────────┬─────────────────╮         │ │
            │                  │ │ nu-cli        │ │ path    │ ./crates/nu-cli │         │ │
            │                  │ │               │ │ version │ 0.74.1          │         │ │
            │                  │ │               │ ╰─────────┴─────────────────╯         │ │
            │                  │ │               │ ╭────────────┬──────────────────────╮ │ │
            │                  │ │ nu-engine     │ │ path       │ ./crates/nu-engine   │ │ │
            │                  │ │               │ │ version    │ 0.74.1               │ │ │
            │                  │ │               │ ╰────────────┴──────────────────────╯ │ │
            │                  │ │               │ ╭─────────────┬─────────────────────╮ │ │
            │                  │ │ reedline      │ │ version     │ 0.14.0              │ │ │
            │                  │ │               │ │             │ ╭───┬──────────╮    │ │ │
            │                  │ │               │ │ features    │ │ 0 │ bashisms │    │ │ │
            │                  │ │               │ │             │ │ 1 │ sqlite   │    │ │ │
            │                  │ │               │ │             │ ╰───┴──────────╯    │ │ │
            │                  │ │               │ ╰─────────────┴─────────────────────╯ │ │
            │                  │ │ rayon         │ 1.6.1                                 │ │
            │                  │ │ is_executable │ 1.0.1                                 │ │
            │                  │ │ simplelog     │ 0.12.0                                │ │
            │                  │ │ time          │ 0.3.12                                │ │
            │                  │ ╰───────────────┴───────────────────────────────────────╯ │
            │                  │ ╭───────────────────────────────────┬───────────────────╮ │
            │ target           │ │ cfg(not(target_os = "windows"))   │ {record 1 field}  │ │
            │                  │ │ cfg(windows)                      │ {record 1 field}  │ │
            │                  │ │ cfg(target_family = "unix")       │ {record 1 field}  │ │
            │                  │ ╰───────────────────────────────────┴───────────────────╯ │
            │                  │ ╭───────────────────┬───────────────────────────────────╮ │
            │ dev-dependencies │ │                   │ ╭─────────┬─────────────────────╮ │ │
            │                  │ │ nu-test-support   │ │ path    │ ./crates/nu-test-su │ │ │
            │                  │ │                   │ │         │ pport               │ │ │
            │                  │ │                   │ │ version │ 0.74.1              │ │ │
            │                  │ │                   │ ╰─────────┴─────────────────────╯ │ │
            │                  │ │ tempfile          │ 3.2.0                             │ │
            │                  │ │ assert_cmd        │ 2.0.2                             │ │
            │                  │ │ criterion         │ 0.4                               │ │
            │                  │ │ pretty_assertions │ 1.0.0                             │ │
            │                  │ │ serial_test       │ 0.10.0                            │ │
            │                  │ │ hamcrest2         │ 0.3.0                             │ │
            │                  │ │                   │ ╭────────────────────┬──────────╮ │ │
            │                  │ │ rstest            │ │ version            │ 0.15.0   │ │ │
            │                  │ │                   │ │ default-features   │ false    │ │ │
            │                  │ │                   │ ╰────────────────────┴──────────╯ │ │
            │                  │ │ itertools         │ 0.10.3                            │ │
            │                  │ ╰───────────────────┴───────────────────────────────────╯ │
            │                  │ ╭─────────────────────┬─────────────────────────────────╮ │
            │ features         │ │                     │ ╭───┬────────────────────╮      │ │
            │                  │ │ plugin              │ │ 0 │ nu-plugin          │      │ │
            │                  │ │                     │ │ 1 │ nu-cli/plugin      │      │ │
            │                  │ │                     │ │ 2 │ nu-parser/plugin   │      │ │
            │                  │ │                     │ │ 3 │ nu-command/plugin  │      │ │
            │                  │ │                     │ │ 4 │ nu-protocol/plugin │      │ │
            │                  │ │                     │ │ 5 │ nu-engine/plugin   │      │ │
            │                  │ │                     │ ╰───┴────────────────────╯      │ │
            │                  │ │                     │ ╭───┬─────────╮                 │ │
            │                  │ │ extra               │ │ 0 │ default │                 │ │
            │                  │ │                     │ ╰───┴─────────╯                 │ │
            │                  │ │                     │ ╭───┬───────────────╮           │ │
            │                  │ │ default             │ │ 0 │ plugin        │           │ │
            │                  │ │                     │ │ 1 │ which-support │           │ │
            │                  │ │                     │ │ 2 │ trash-support │           │ │
            │                  │ │                     │ │ 3 │ sqlite        │           │ │
            │                  │ │                     │ ╰───┴───────────────╯           │ │
            │                  │ │                     │ ╭───┬─────────╮                 │ │
            │                  │ │ stable              │ │ 0 │ default │                 │ │
            │                  │ │                     │ ╰───┴─────────╯                 │ │
            │                  │ │ wasi                │ [list 0 items]                  │ │
            │                  │ │                     │ ╭───┬─────────────╮             │ │
            │                  │ │ static-link-openssl │ │ 0 │ dep:openssl │             │ │
            │                  │ │                     │ ╰───┴─────────────╯             │ │
            │                  │ │                     │ ╭───┬─────────────────────────╮ │ │
            │                  │ │ which-support       │ │ 0 │ nu-command/which-suppor │ │ │
            │                  │ │                     │ │   │ t                       │ │ │
            │                  │ │                     │ ╰───┴─────────────────────────╯ │ │
            │                  │ │                     │ ╭───┬─────────────────────────╮ │ │
            │                  │ │ trash-support       │ │ 0 │ nu-command/trash-suppor │ │ │
            │                  │ │                     │ │   │ t                       │ │ │
            │                  │ │                     │ ╰───┴─────────────────────────╯ │ │
            │                  │ ╰─────────────────────┴─────────────────────────────────╯ │
            │                  │ ╭───┬──────┬─────────────╮                                │
            │ bin              │ │ # │ name │    path     │                                │
            │                  │ ├───┼──────┼─────────────┤                                │
            │                  │ │ 0 │ nu   │ src/main.rs │                                │
            │                  │ ╰───┴──────┴─────────────╯                                │
            │                  │ ╭───────────┬───────────────────────────────────────────╮ │
            │ patch            │ │           │ ╭──────────┬────────────────────────────╮ │ │
            │                  │ │ crates-io │ │          │ ╭────────┬───────────────╮ │ │ │
            │                  │ │           │ │ reedline │ │ git    │ https://githu │ │ │ │
            │                  │ │           │ │          │ │        │ b.com/nushell │ │ │ │
            │                  │ │           │ │          │ │        │ /reedline.git │ │ │ │
            │                  │ │           │ │          │ │ branch │ main          │ │ │ │
            │                  │ │           │ │          │ ╰────────┴───────────────╯ │ │ │
            │                  │ │           │ ╰──────────┴────────────────────────────╯ │ │
            │                  │ ╰───────────┴───────────────────────────────────────────╯ │
            │                  │ ╭───┬────────────┬─────────╮                              │
            │ bench            │ │ # │    name    │ harness │                              │
            │                  │ ├───┼────────────┼─────────┤                              │
            │                  │ │ 0 │ benchmarks │ false   │                              │
            │                  │ ╰───┴────────────┴─────────╯                              │
            ╰──────────────────┴───────────────────────────────────────────────────────────╯"#};
        assert_eq!(actual, expected);
        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --expand --width=120")?;
        let expected = indoc! {r#"
            ╭──────────────────┬───────────────────────────────────────────────────────────────────────────────────────────────────╮
            │                  │ ╭───────────────┬───────────────────────────────────────────────────────────────────────────────╮ │
            │ package          │ │               │ ╭───┬────────────────────────────────╮                                        │ │
            │                  │ │ authors       │ │ 0 │ The Nushell Project Developers │                                        │ │
            │                  │ │               │ ╰───┴────────────────────────────────╯                                        │ │
            │                  │ │ default-run   │ nu                                                                            │ │
            │                  │ │ description   │ A new type of shell                                                           │ │
            │                  │ │ documentation │ https://www.nushell.sh/book/                                                  │ │
            │                  │ │ edition       │ 2024                                                                          │ │
            │                  │ │               │ ╭───┬────────╮                                                                │ │
            │                  │ │ exclude       │ │ 0 │ images │                                                                │ │
            │                  │ │               │ ╰───┴────────╯                                                                │ │
            │                  │ │ homepage      │ https://www.nushell.sh                                                        │ │
            │                  │ │ license       │ MIT                                                                           │ │
            │                  │ │ name          │ nu                                                                            │ │
            │                  │ │ repository    │ https://github.com/nushell/nushell                                            │ │
            │                  │ │ rust-version  │ 1.60                                                                          │ │
            │                  │ │ version       │ 0.74.1                                                                        │ │
            │                  │ │               │ ╭──────────┬────────────────────────────────────────────────────────────────╮ │ │
            │                  │ │ metadata      │ │          │ ╭───────────┬────────────────────────────────────────────────╮ │ │ │
            │                  │ │               │ │ binstall │ │ pkg-url   │ { repo }/releases/download/{ version }/{ name  │ │ │ │
            │                  │ │               │ │          │ │           │ }-{ version }-{ target }.{ archive-format }    │ │ │ │
            │                  │ │               │ │          │ │ pkg-fmt   │ tgz                                            │ │ │ │
            │                  │ │               │ │          │ │           │ ╭─────────────────────────┬──────────────────╮ │ │ │ │
            │                  │ │               │ │          │ │ overrides │ │ x86_64-pc-windows-msvc  │ {record 1 field} │ │ │ │ │
            │                  │ │               │ │          │ │           │ ╰─────────────────────────┴──────────────────╯ │ │ │ │
            │                  │ │               │ │          │ ╰───────────┴────────────────────────────────────────────────╯ │ │ │
            │                  │ │               │ ╰──────────┴────────────────────────────────────────────────────────────────╯ │ │
            │                  │ ╰───────────────┴───────────────────────────────────────────────────────────────────────────────╯ │
            │                  │ ╭─────────┬─────────────────────────────────────────╮                                             │
            │ workspace        │ │         │ ╭────┬────────────────────────────────╮ │                                             │
            │                  │ │ members │ │  0 │ crates/nu-cli                  │ │                                             │
            │                  │ │         │ │  1 │ crates/nu-engine               │ │                                             │
            │                  │ │         │ │  2 │ crates/nu-parser               │ │                                             │
            │                  │ │         │ │  3 │ crates/nu-system               │ │                                             │
            │                  │ │         │ │  4 │ crates/nu-command              │ │                                             │
            │                  │ │         │ │  5 │ crates/nu-protocol             │ │                                             │
            │                  │ │         │ │  6 │ crates/nu-plugin               │ │                                             │
            │                  │ │         │ │  7 │ crates/nu_plugin_inc           │ │                                             │
            │                  │ │         │ │  8 │ crates/nu_plugin_gstat         │ │                                             │
            │                  │ │         │ │  9 │ crates/nu_plugin_example       │ │                                             │
            │                  │ │         │ │ 10 │ crates/nu_plugin_query         │ │                                             │
            │                  │ │         │ │ 11 │ crates/nu_plugin_custom_values │ │                                             │
            │                  │ │         │ │ 12 │ crates/nu-utils                │ │                                             │
            │                  │ │         │ ╰────┴────────────────────────────────╯ │                                             │
            │                  │ ╰─────────┴─────────────────────────────────────────╯                                             │
            │                  │ ╭───────────────┬───────────────────────────────────────────╮                                     │
            │ dependencies     │ │               │ ╭──────────┬───────────────╮              │                                     │
            │                  │ │ chrono        │ │ version  │ 0.4.23        │              │                                     │
            │                  │ │               │ │          │ ╭───┬───────╮ │              │                                     │
            │                  │ │               │ │ features │ │ 0 │ serde │ │              │                                     │
            │                  │ │               │ │          │ ╰───┴───────╯ │              │                                     │
            │                  │ │               │ ╰──────────┴───────────────╯              │                                     │
            │                  │ │ crossterm     │ 0.24.0                                    │                                     │
            │                  │ │ ctrlc         │ 3.2.1                                     │                                     │
            │                  │ │ log           │ 0.4                                       │                                     │
            │                  │ │               │ ╭──────────┬────────────────────────────╮ │                                     │
            │                  │ │ miette        │ │ version  │ 5.5.0                      │ │                                     │
            │                  │ │               │ │          │ ╭───┬────────────────────╮ │ │                                     │
            │                  │ │               │ │ features │ │ 0 │ fancy-no-backtrace │ │ │                                     │
            │                  │ │               │ │          │ ╰───┴────────────────────╯ │ │                                     │
            │                  │ │               │ ╰──────────┴────────────────────────────╯ │                                     │
            │                  │ │ nu-ansi-term  │ 0.46.0                                    │                                     │
            │                  │ │               │ ╭─────────┬─────────────────╮             │                                     │
            │                  │ │ nu-cli        │ │ path    │ ./crates/nu-cli │             │                                     │
            │                  │ │               │ │ version │ 0.74.1          │             │                                     │
            │                  │ │               │ ╰─────────┴─────────────────╯             │                                     │
            │                  │ │               │ ╭─────────┬────────────────────╮          │                                     │
            │                  │ │ nu-engine     │ │ path    │ ./crates/nu-engine │          │                                     │
            │                  │ │               │ │ version │ 0.74.1             │          │                                     │
            │                  │ │               │ ╰─────────┴────────────────────╯          │                                     │
            │                  │ │               │ ╭──────────┬──────────────────╮           │                                     │
            │                  │ │ reedline      │ │ version  │ 0.14.0           │           │                                     │
            │                  │ │               │ │          │ ╭───┬──────────╮ │           │                                     │
            │                  │ │               │ │ features │ │ 0 │ bashisms │ │           │                                     │
            │                  │ │               │ │          │ │ 1 │ sqlite   │ │           │                                     │
            │                  │ │               │ │          │ ╰───┴──────────╯ │           │                                     │
            │                  │ │               │ ╰──────────┴──────────────────╯           │                                     │
            │                  │ │ rayon         │ 1.6.1                                     │                                     │
            │                  │ │ is_executable │ 1.0.1                                     │                                     │
            │                  │ │ simplelog     │ 0.12.0                                    │                                     │
            │                  │ │ time          │ 0.3.12                                    │                                     │
            │                  │ ╰───────────────┴───────────────────────────────────────────╯                                     │
            │                  │ ╭─────────────────────────────────┬─────────────────────────────────────────────────────────────╮ │
            │ target           │ │                                 │ ╭──────────────┬──────────────────────────────────────────╮ │ │
            │                  │ │ cfg(not(target_os = "windows")) │ │              │ ╭─────────────┬────────────────────────╮ │ │ │
            │                  │ │                                 │ │ dependencies │ │             │ ╭──────────┬─────────╮ │ │ │ │
            │                  │ │                                 │ │              │ │ openssl     │ │ version  │ 0.10.38 │ │ │ │ │
            │                  │ │                                 │ │              │ │             │ │ features │ [list 1 │ │ │ │ │
            │                  │ │                                 │ │              │ │             │ │          │  item]  │ │ │ │ │
            │                  │ │                                 │ │              │ │             │ │ optional │ true    │ │ │ │ │
            │                  │ │                                 │ │              │ │             │ ╰──────────┴─────────╯ │ │ │ │
            │                  │ │                                 │ │              │ │ signal-hook │ {record 2 fields}      │ │ │ │
            │                  │ │                                 │ │              │ ╰─────────────┴────────────────────────╯ │ │ │
            │                  │ │                                 │ ╰──────────────┴──────────────────────────────────────────╯ │ │
            │                  │ │                                 │ ╭────────────────────┬──────────────────╮                   │ │
            │                  │ │ cfg(windows)                    │ │                    │ ╭────────┬─────╮ │                   │ │
            │                  │ │                                 │ │ build-dependencies │ │ winres │ 0.1 │ │                   │ │
            │                  │ │                                 │ │                    │ ╰────────┴─────╯ │                   │ │
            │                  │ │                                 │ ╰────────────────────┴──────────────────╯                   │ │
            │                  │ │                                 │ ╭──────────────┬──────────────────────────────────────────╮ │ │
            │                  │ │ cfg(target_family = "unix")     │ │              │ ╭──────┬───────────────────────────────╮ │ │ │
            │                  │ │                                 │ │ dependencies │ │      │ ╭──────────────────┬────────╮ │ │ │ │
            │                  │ │                                 │ │              │ │ nix  │ │ version          │ 0.25   │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ │ default-features │ false  │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ │ features         │ [list  │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ │                  │ 4      │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ │                  │ items] │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ ╰──────────────────┴────────╯ │ │ │ │
            │                  │ │                                 │ │              │ │ atty │ 0.2                           │ │ │ │
            │                  │ │                                 │ │              │ ╰──────┴───────────────────────────────╯ │ │ │
            │                  │ │                                 │ ╰──────────────┴──────────────────────────────────────────╯ │ │
            │                  │ ╰─────────────────────────────────┴─────────────────────────────────────────────────────────────╯ │
            │                  │ ╭───────────────────┬────────────────────────────────────────╮                                    │
            │ dev-dependencies │ │                   │ ╭─────────┬──────────────────────────╮ │                                    │
            │                  │ │ nu-test-support   │ │ path    │ ./crates/nu-test-support │ │                                    │
            │                  │ │                   │ │ version │ 0.74.1                   │ │                                    │
            │                  │ │                   │ ╰─────────┴──────────────────────────╯ │                                    │
            │                  │ │ tempfile          │ 3.2.0                                  │                                    │
            │                  │ │ assert_cmd        │ 2.0.2                                  │                                    │
            │                  │ │ criterion         │ 0.4                                    │                                    │
            │                  │ │ pretty_assertions │ 1.0.0                                  │                                    │
            │                  │ │ serial_test       │ 0.10.0                                 │                                    │
            │                  │ │ hamcrest2         │ 0.3.0                                  │                                    │
            │                  │ │                   │ ╭──────────────────┬────────╮          │                                    │
            │                  │ │ rstest            │ │ version          │ 0.15.0 │          │                                    │
            │                  │ │                   │ │ default-features │ false  │          │                                    │
            │                  │ │                   │ ╰──────────────────┴────────╯          │                                    │
            │                  │ │ itertools         │ 0.10.3                                 │                                    │
            │                  │ ╰───────────────────┴────────────────────────────────────────╯                                    │
            │                  │ ╭─────────────────────┬──────────────────────────────────╮                                        │
            │ features         │ │                     │ ╭───┬────────────────────╮       │                                        │
            │                  │ │ plugin              │ │ 0 │ nu-plugin          │       │                                        │
            │                  │ │                     │ │ 1 │ nu-cli/plugin      │       │                                        │
            │                  │ │                     │ │ 2 │ nu-parser/plugin   │       │                                        │
            │                  │ │                     │ │ 3 │ nu-command/plugin  │       │                                        │
            │                  │ │                     │ │ 4 │ nu-protocol/plugin │       │                                        │
            │                  │ │                     │ │ 5 │ nu-engine/plugin   │       │                                        │
            │                  │ │                     │ ╰───┴────────────────────╯       │                                        │
            │                  │ │                     │ ╭───┬─────────╮                  │                                        │
            │                  │ │ extra               │ │ 0 │ default │                  │                                        │
            │                  │ │                     │ ╰───┴─────────╯                  │                                        │
            │                  │ │                     │ ╭───┬───────────────╮            │                                        │
            │                  │ │ default             │ │ 0 │ plugin        │            │                                        │
            │                  │ │                     │ │ 1 │ which-support │            │                                        │
            │                  │ │                     │ │ 2 │ trash-support │            │                                        │
            │                  │ │                     │ │ 3 │ sqlite        │            │                                        │
            │                  │ │                     │ ╰───┴───────────────╯            │                                        │
            │                  │ │                     │ ╭───┬─────────╮                  │                                        │
            │                  │ │ stable              │ │ 0 │ default │                  │                                        │
            │                  │ │                     │ ╰───┴─────────╯                  │                                        │
            │                  │ │ wasi                │ [list 0 items]                   │                                        │
            │                  │ │                     │ ╭───┬─────────────╮              │                                        │
            │                  │ │ static-link-openssl │ │ 0 │ dep:openssl │              │                                        │
            │                  │ │                     │ ╰───┴─────────────╯              │                                        │
            │                  │ │                     │ ╭───┬──────────────────────────╮ │                                        │
            │                  │ │ which-support       │ │ 0 │ nu-command/which-support │ │                                        │
            │                  │ │                     │ ╰───┴──────────────────────────╯ │                                        │
            │                  │ │                     │ ╭───┬──────────────────────────╮ │                                        │
            │                  │ │ trash-support       │ │ 0 │ nu-command/trash-support │ │                                        │
            │                  │ │                     │ ╰───┴──────────────────────────╯ │                                        │
            │                  │ ╰─────────────────────┴──────────────────────────────────╯                                        │
            │                  │ ╭───┬──────┬─────────────╮                                                                        │
            │ bin              │ │ # │ name │    path     │                                                                        │
            │                  │ ├───┼──────┼─────────────┤                                                                        │
            │                  │ │ 0 │ nu   │ src/main.rs │                                                                        │
            │                  │ ╰───┴──────┴─────────────╯                                                                        │
            │                  │ ╭───────────┬───────────────────────────────────────────────────────────────────────────────────╮ │
            │ patch            │ │           │ ╭─────────────────┬─────────────────────────────────────────────────────────────╮ │ │
            │                  │ │ crates-io │ │                 │ ╭────────┬─────────────────────────────────────────╮        │ │ │
            │                  │ │           │ │ reedline        │ │ git    │ https://github.com/nushell/reedline.git │        │ │ │
            │                  │ │           │ │                 │ │ branch │ main                                    │        │ │ │
            │                  │ │           │ │                 │ ╰────────┴─────────────────────────────────────────╯        │ │ │
            │                  │ │           │ ╰─────────────────┴─────────────────────────────────────────────────────────────╯ │ │
            │                  │ ╰───────────┴───────────────────────────────────────────────────────────────────────────────────╯ │
            │                  │ ╭───┬────────────┬─────────╮                                                                      │
            │ bench            │ │ # │    name    │ harness │                                                                      │
            │                  │ ├───┼────────────┼─────────┤                                                                      │
            │                  │ │ 0 │ benchmarks │ false   │                                                                      │
            │                  │ ╰───┴────────────┴─────────╯                                                                      │
            ╰──────────────────┴───────────────────────────────────────────────────────────────────────────────────────────────────╯"#};
        assert_eq!(actual, expected);
        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --expand --width=60")?;
        let expected = indoc! {"
            ╭──────────────────┬───────────────────────────────────────╮
            │                  │ ╭───────────────┬───────────────────╮ │
            │ package          │ │               │ ╭───┬───────────╮ │ │
            │                  │ │ authors       │ │ 0 │ The       │ │ │
            │                  │ │               │ │   │ Nushell   │ │ │
            │                  │ │               │ │   │ Project D │ │ │
            │                  │ │               │ │   │ evelopers │ │ │
            │                  │ │               │ ╰───┴───────────╯ │ │
            │                  │ │ default-run   │ nu                │ │
            │                  │ │ description   │ A new type of     │ │
            │                  │ │               │ shell             │ │
            │                  │ │ documentation │ https://www.nushe │ │
            │                  │ │               │ ll.sh/book/       │ │
            │                  │ │ edition       │ 2024              │ │
            │                  │ │               │ ╭───┬────────╮    │ │
            │                  │ │ exclude       │ │ 0 │ images │    │ │
            │                  │ │               │ ╰───┴────────╯    │ │
            │                  │ │ homepage      │ https://www.nushe │ │
            │                  │ │               │ ll.sh             │ │
            │                  │ │ license       │ MIT               │ │
            │                  │ │ name          │ nu                │ │
            │                  │ │ repository    │ https://github.co │ │
            │                  │ │               │ m/nushell/nushell │ │
            │                  │ │ rust-version  │ 1.60              │ │
            │                  │ │ version       │ 0.74.1            │ │
            │                  │ │ metadata      │ {record 1 field}  │ │
            │                  │ ╰───────────────┴───────────────────╯ │
            │                  │ ╭─────────┬─────────────────────────╮ │
            │ workspace        │ │         │ ╭────┬────────────────╮ │ │
            │                  │ │ members │ │  0 │ crates/nu-cli  │ │ │
            │                  │ │         │ │  1 │ crates/nu-engi │ │ │
            │                  │ │         │ │    │ ne             │ │ │
            │                  │ │         │ │  2 │ crates/nu-pars │ │ │
            │                  │ │         │ │    │ er             │ │ │
            │                  │ │         │ │  3 │ crates/nu-syst │ │ │
            │                  │ │         │ │    │ em             │ │ │
            │                  │ │         │ │  4 │ crates/nu-comm │ │ │
            │                  │ │         │ │    │ and            │ │ │
            │                  │ │         │ │  5 │ crates/nu-prot │ │ │
            │                  │ │         │ │    │ ocol           │ │ │
            │                  │ │         │ │  6 │ crates/nu-plug │ │ │
            │                  │ │         │ │    │ in             │ │ │
            │                  │ │         │ │  7 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_inc         │ │ │
            │                  │ │         │ │  8 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_gstat       │ │ │
            │                  │ │         │ │  9 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_example     │ │ │
            │                  │ │         │ │ 10 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_query       │ │ │
            │                  │ │         │ │ 11 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_custom_valu │ │ │
            │                  │ │         │ │    │ es             │ │ │
            │                  │ │         │ │ 12 │ crates/nu-util │ │ │
            │                  │ │         │ │    │ s              │ │ │
            │                  │ │         │ ╰────┴────────────────╯ │ │
            │                  │ ╰─────────┴─────────────────────────╯ │
            │                  │ ╭───────────────┬───────────────────╮ │
            │ dependencies     │ │ chrono        │ {record 2 fields} │ │
            │                  │ │ crossterm     │ 0.24.0            │ │
            │                  │ │ ctrlc         │ 3.2.1             │ │
            │                  │ │ log           │ 0.4               │ │
            │                  │ │ miette        │ {record 2 fields} │ │
            │                  │ │ nu-ansi-term  │ 0.46.0            │ │
            │                  │ │ nu-cli        │ {record 2 fields} │ │
            │                  │ │ nu-engine     │ {record 2 fields} │ │
            │                  │ │ reedline      │ {record 2 fields} │ │
            │                  │ │ rayon         │ 1.6.1             │ │
            │                  │ │ is_executable │ 1.0.1             │ │
            │                  │ │ simplelog     │ 0.12.0            │ │
            │                  │ │ time          │ 0.3.12            │ │
            │                  │ ╰───────────────┴───────────────────╯ │
            │ target           │ {record 3 fields}                     │
            │                  │ ╭─────────────────────┬─────────────╮ │
            │ dev-dependencies │ │ nu-test-support     │ {record 2   │ │
            │                  │ │                     │ fields}     │ │
            │                  │ │ tempfile            │ 3.2.0       │ │
            │                  │ │ assert_cmd          │ 2.0.2       │ │
            │                  │ │ criterion           │ 0.4         │ │
            │                  │ │ pretty_assertions   │ 1.0.0       │ │
            │                  │ │ serial_test         │ 0.10.0      │ │
            │                  │ │ hamcrest2           │ 0.3.0       │ │
            │                  │ │ rstest              │ {record 2   │ │
            │                  │ │                     │ fields}     │ │
            │                  │ │ itertools           │ 0.10.3      │ │
            │                  │ ╰─────────────────────┴─────────────╯ │
            │                  │ ╭─────────────────────┬─────────────╮ │
            │ features         │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ plugin              │ │ 0 │ nu- │ │ │
            │                  │ │                     │ │   │ plu │ │ │
            │                  │ │                     │ │   │ gin │ │ │
            │                  │ │                     │ │ 1 │ nu- │ │ │
            │                  │ │                     │ │   │ cli │ │ │
            │                  │ │                     │ │   │ /pl │ │ │
            │                  │ │                     │ │   │ ugi │ │ │
            │                  │ │                     │ │   │ n   │ │ │
            │                  │ │                     │ │ 2 │ nu- │ │ │
            │                  │ │                     │ │   │ par │ │ │
            │                  │ │                     │ │   │ ser │ │ │
            │                  │ │                     │ │   │ /pl │ │ │
            │                  │ │                     │ │   │ ugi │ │ │
            │                  │ │                     │ │   │ n   │ │ │
            │                  │ │                     │ │ 3 │ nu- │ │ │
            │                  │ │                     │ │   │ com │ │ │
            │                  │ │                     │ │   │ man │ │ │
            │                  │ │                     │ │   │ d/p │ │ │
            │                  │ │                     │ │   │ lug │ │ │
            │                  │ │                     │ │   │ in  │ │ │
            │                  │ │                     │ │ 4 │ nu- │ │ │
            │                  │ │                     │ │   │ pro │ │ │
            │                  │ │                     │ │   │ toc │ │ │
            │                  │ │                     │ │   │ ol/ │ │ │
            │                  │ │                     │ │   │ plu │ │ │
            │                  │ │                     │ │   │ gin │ │ │
            │                  │ │                     │ │ 5 │ nu- │ │ │
            │                  │ │                     │ │   │ eng │ │ │
            │                  │ │                     │ │   │ ine │ │ │
            │                  │ │                     │ │   │ /pl │ │ │
            │                  │ │                     │ │   │ ugi │ │ │
            │                  │ │                     │ │   │ n   │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ extra               │ │ 0 │ def │ │ │
            │                  │ │                     │ │   │ aul │ │ │
            │                  │ │                     │ │   │ t   │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ default             │ │ 0 │ plu │ │ │
            │                  │ │                     │ │   │ gin │ │ │
            │                  │ │                     │ │ 1 │ whi │ │ │
            │                  │ │                     │ │   │ ch- │ │ │
            │                  │ │                     │ │   │ sup │ │ │
            │                  │ │                     │ │   │ por │ │ │
            │                  │ │                     │ │   │ t   │ │ │
            │                  │ │                     │ │ 2 │ tra │ │ │
            │                  │ │                     │ │   │ sh- │ │ │
            │                  │ │                     │ │   │ sup │ │ │
            │                  │ │                     │ │   │ por │ │ │
            │                  │ │                     │ │   │ t   │ │ │
            │                  │ │                     │ │ 3 │ sql │ │ │
            │                  │ │                     │ │   │ ite │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ stable              │ │ 0 │ def │ │ │
            │                  │ │                     │ │   │ aul │ │ │
            │                  │ │                     │ │   │ t   │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │ wasi                │ [list 0     │ │
            │                  │ │                     │ items]      │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ static-link-openssl │ │ 0 │ dep │ │ │
            │                  │ │                     │ │   │ :op │ │ │
            │                  │ │                     │ │   │ ens │ │ │
            │                  │ │                     │ │   │ sl  │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ which-support       │ │ 0 │ nu- │ │ │
            │                  │ │                     │ │   │ com │ │ │
            │                  │ │                     │ │   │ man │ │ │
            │                  │ │                     │ │   │ d/w │ │ │
            │                  │ │                     │ │   │ hic │ │ │
            │                  │ │                     │ │   │ h-s │ │ │
            │                  │ │                     │ │   │ upp │ │ │
            │                  │ │                     │ │   │ ort │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ trash-support       │ │ 0 │ nu- │ │ │
            │                  │ │                     │ │   │ com │ │ │
            │                  │ │                     │ │   │ man │ │ │
            │                  │ │                     │ │   │ d/t │ │ │
            │                  │ │                     │ │   │ ras │ │ │
            │                  │ │                     │ │   │ h-s │ │ │
            │                  │ │                     │ │   │ upp │ │ │
            │                  │ │                     │ │   │ ort │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ ╰─────────────────────┴─────────────╯ │
            │                  │ ╭───┬──────┬─────────────╮            │
            │ bin              │ │ # │ name │    path     │            │
            │                  │ ├───┼──────┼─────────────┤            │
            │                  │ │ 0 │ nu   │ src/main.rs │            │
            │                  │ ╰───┴──────┴─────────────╯            │
            │                  │ ╭───────────┬───────────────────────╮ │
            │ patch            │ │           │ ╭──────────┬────────╮ │ │
            │                  │ │ crates-io │ │ reedline │ {recor │ │ │
            │                  │ │           │ │          │ d 2 fi │ │ │
            │                  │ │           │ │          │ elds}  │ │ │
            │                  │ │           │ ╰──────────┴────────╯ │ │
            │                  │ ╰───────────┴───────────────────────╯ │
            │                  │ ╭───┬────────────┬─────────╮          │
            │ bench            │ │ # │    name    │ harness │          │
            │                  │ ├───┼────────────┼─────────┤          │
            │                  │ │ 0 │ benchmarks │ false   │          │
            │                  │ ╰───┴────────────┴─────────╯          │
            ╰──────────────────┴───────────────────────────────────────╯"};
        assert_eq!(actual, expected);
        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --expand --width=40")?;
        let expected = indoc! {"
            ╭──────────────────┬───────────────────╮
            │ package          │ {record 13        │
            │                  │ fields}           │
            │ workspace        │ {record 1 field}  │
            │ dependencies     │ {record 13        │
            │                  │ fields}           │
            │ target           │ {record 3 fields} │
            │ dev-dependencies │ {record 9 fields} │
            │ features         │ {record 8 fields} │
            │                  │ ╭───┬──────╮      │
            │ bin              │ │ # │ name │      │
            │                  │ ├───┼──────┤      │
            │                  │ │ 0 │ nu   │      │
            │                  │ ╰───┴──────╯      │
            │ patch            │ {record 1 field}  │
            │                  │ ╭───┬──────╮      │
            │ bench            │ │ # │ name │      │
            │                  │ ├───┼──────┤      │
            │                  │ │ 0 │ benc │      │
            │                  │ │   │ hmar │      │
            │                  │ │   │ ks   │      │
            │                  │ ╰───┴──────╯      │
            ╰──────────────────┴───────────────────╯"};
        assert_eq!(actual, expected);
        Ok(())
    })
}

#[test]
fn table_expande_with_no_header_internally_0() -> Result {
    let nu_value = r##"{ "config            ": { "ls": { "use_ls_colors": true, "clickable_links": false }, "rm": { "always_trash": false }, "cd": { "abbreviations": false }, "table": { "mode": "rounded", "index_mode": "always", "trim": { "methodology": "wrapping", "wrapping_try_keep_words": true, "truncating_suffix": "..." } }, "explore": { "help_banner": true, "exit_esc": true, "command_bar_text": "#C4C9C6", "status_bar_background": { "fg": "#1D1F21", "bg": "#C4C9C6" }, "highlight": { "bg": "yellow", "fg": "black" }, "status": {}, "try": {}, "table": { "split_line": "#404040", "cursor": true, "line_index": true, "line_shift": true, "line_head_top": true, "line_head_bottom": true, "show_head": true, "show_index": true }, "config": { "cursor_color": { "bg": "yellow", "fg": "black" } } }, "history": { "max_size": 10000, "sync_on_enter": true, "file_format": "plaintext" }, "completions": { "case_sensitive": false, "quick": true, "partial": true, "algorithm": "prefix", "external": { "enable": true, "max_results": 100, "completer": null } }, "filesize": { "metric": true, "format": "auto" }, "cursor_shape": { "emacs": "line", "vi_insert": "block", "vi_normal": "underscore" }, "color_config": { "separator": "default", "leading_trailing_space_bg": { "attr": "n" }, "header": "green_bold", "empty": "blue", "bool": null, "int": "default", "filesize": null, "duration": "default", "datetime": null, "range": "default", "float": "default", "string": "default", "nothing": "default", "binary": "default", "cell-path": "default", "row_index": "green_bold", "record": "default", "list": "default", "block": "default", "hints": "dark_gray", "search_result": {"fg": "white", "bg": "red"}, "shape_and": "purple_bold", "shape_binary": "purple_bold", "shape_block": "blue_bold", "shape_bool": "light_cyan", "shape_custom": "green", "shape_datetime": "cyan_bold", "shape_directory": "cyan", "shape_external": "cyan", "shape_externalarg": "green_bold", "shape_filepath": "cyan", "shape_flag": "blue_bold", "shape_float": "purple_bold", "shape_garbage": { "fg": "#FFFFFF", "bg": "#FF0000", "attr": "b" }, "shape_globpattern": "cyan_bold", "shape_int": "purple_bold", "shape_internalcall": "cyan_bold", "shape_list": "cyan_bold", "shape_literal": "blue", "shape_matching_brackets": { "attr": "u" }, "shape_nothing": "light_cyan", "shape_operator": "yellow", "shape_or": "purple_bold", "shape_pipe": "purple_bold", "shape_range": "yellow_bold", "shape_record": "cyan_bold", "shape_redirection": "purple_bold", "shape_signature": "green_bold", "shape_string": "green", "shape_string_interpolation": "cyan_bold", "shape_table": "blue_bold", "shape_variable": "purple" }, "footer_mode": "25", "float_precision": 2, "use_ansi_coloring": true, "edit_mode": "emacs", "shell_integration": true, "show_banner": true, "render_right_prompt_on_last_line": false, "hooks": { "pre_prompt": [ null ], "pre_execution": [ null ], "env_change": { "PWD": [ null ] }, "display_output": null }, "menus": [ { "name": "completion_menu", "only_buffer_difference": false, "marker": "| ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "history_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "help_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "commands_menu", "only_buffer_difference": false, "marker": "# ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "vars_menu", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "commands_with_description", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null } ], "keybindings": [ { "name": "completion_menu", "modifier": "none", "keycode": "tab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "send": "menu", "name": "completion_menu" }, { "send": "menunext" } ] } }, { "name": "completion_previous", "modifier": "shift", "keycode": "backtab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menuprevious" } }, { "name": "history_menu", "modifier": "control", "keycode": "char_r", "mode": "emacs", "event": { "send": "menu", "name": "history_menu" } }, { "name": "next_page", "modifier": "control", "keycode": "char_x", "mode": "emacs", "event": { "send": "menupagenext" } }, { "name": "undo_or_previous_page", "modifier": "control", "keycode": "char_z", "mode": "emacs", "event": { "until": [ { "send": "menupageprevious" }, { "edit": "undo" } ] } }, { "name": "yank", "modifier": "control", "keycode": "char_y", "mode": "emacs", "event": { "until": [ { "edit": "pastecutbufferafter" } ] } }, { "name": "unix-line-discard", "modifier": "control", "keycode": "char_u", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cutfromlinestart" } ] } }, { "name": "kill-line", "modifier": "control", "keycode": "char_k", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cuttolineend" } ] } }, { "name": "commands_menu", "modifier": "control", "keycode": "char_t", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_menu" } }, { "name": "vars_menu", "modifier": "alt", "keycode": "char_o", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "vars_menu" } }, { "name": "commands_with_description", "modifier": "control", "keycode": "char_s", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_with_description" } } ] } }"##;
    let mut tester = test();
    let data: Value = tester.run(nu_value.trim())?;
    let actual: String = tester.run_with_data("table --expand --width 141", data)?;
    assert_eq!(
        actual,
        indoc! {"
            ╭────────────────────┬──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
            │                    │ ╭──────────────────────────────────┬───────────────────────────────────────────────────────────────────────────────╮ │
            │ config             │ │                                  │ ╭─────────────────┬───────╮                                                   │ │
            │                    │ │ ls                               │ │ use_ls_colors   │ true  │                                                   │ │
            │                    │ │                                  │ │ clickable_links │ false │                                                   │ │
            │                    │ │                                  │ ╰─────────────────┴───────╯                                                   │ │
            │                    │ │                                  │ ╭──────────────┬───────╮                                                      │ │
            │                    │ │ rm                               │ │ always_trash │ false │                                                      │ │
            │                    │ │                                  │ ╰──────────────┴───────╯                                                      │ │
            │                    │ │                                  │ ╭───────────────┬───────╮                                                     │ │
            │                    │ │ cd                               │ │ abbreviations │ false │                                                     │ │
            │                    │ │                                  │ ╰───────────────┴───────╯                                                     │ │
            │                    │ │                                  │ ╭────────────┬────────────────────────────────────────╮                       │ │
            │                    │ │ table                            │ │ mode       │ rounded                                │                       │ │
            │                    │ │                                  │ │ index_mode │ always                                 │                       │ │
            │                    │ │                                  │ │            │ ╭─────────────────────────┬──────────╮ │                       │ │
            │                    │ │                                  │ │ trim       │ │ methodology             │ wrapping │ │                       │ │
            │                    │ │                                  │ │            │ │ wrapping_try_keep_words │ true     │ │                       │ │
            │                    │ │                                  │ │            │ │ truncating_suffix       │ ...      │ │                       │ │
            │                    │ │                                  │ │            │ ╰─────────────────────────┴──────────╯ │                       │ │
            │                    │ │                                  │ ╰────────────┴────────────────────────────────────────╯                       │ │
            │                    │ │                                  │ ╭───────────────────────────────┬───────────────────────────────────────────╮ │ │
            │                    │ │ explore                          │ │ help_banner                   │ true                                      │ │ │
            │                    │ │                                  │ │ exit_esc                      │ true                                      │ │ │
            │                    │ │                                  │ │ command_bar_text              │ #C4C9C6                                   │ │ │
            │                    │ │                                  │ │                               │ ╭────┬─────────╮                          │ │ │
            │                    │ │                                  │ │ status_bar_background         │ │ fg │ #1D1F21 │                          │ │ │
            │                    │ │                                  │ │                               │ │ bg │ #C4C9C6 │                          │ │ │
            │                    │ │                                  │ │                               │ ╰────┴─────────╯                          │ │ │
            │                    │ │                                  │ │                               │ ╭────┬────────╮                           │ │ │
            │                    │ │                                  │ │ highlight                     │ │ bg │ yellow │                           │ │ │
            │                    │ │                                  │ │                               │ │ fg │ black  │                           │ │ │
            │                    │ │                                  │ │                               │ ╰────┴────────╯                           │ │ │
            │                    │ │                                  │ │ status                        │ {record 0 fields}                         │ │ │
            │                    │ │                                  │ │ try                           │ {record 0 fields}                         │ │ │
            │                    │ │                                  │ │                               │ ╭──────────────────┬─────────╮            │ │ │
            │                    │ │                                  │ │ table                         │ │ split_line       │ #404040 │            │ │ │
            │                    │ │                                  │ │                               │ │ cursor           │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ line_index       │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ line_shift       │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ line_head_top    │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ line_head_bottom │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ show_head        │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ show_index       │ true    │            │ │ │
            │                    │ │                                  │ │                               │ ╰──────────────────┴─────────╯            │ │ │
            │                    │ │                                  │ │                               │ ╭──────────────┬─────────────────╮        │ │ │
            │                    │ │                                  │ │ config                        │ │              │ ╭────┬────────╮ │        │ │ │
            │                    │ │                                  │ │                               │ │ cursor_color │ │ bg │ yellow │ │        │ │ │
            │                    │ │                                  │ │                               │ │              │ │ fg │ black  │ │        │ │ │
            │                    │ │                                  │ │                               │ │              │ ╰────┴────────╯ │        │ │ │
            │                    │ │                                  │ │                               │ ╰──────────────┴─────────────────╯        │ │ │
            │                    │ │                                  │ ╰───────────────────────────────┴───────────────────────────────────────────╯ │ │
            │                    │ │                                  │ ╭───────────────┬───────────╮                                                 │ │
            │                    │ │ history                          │ │ max_size      │ 10000     │                                                 │ │
            │                    │ │                                  │ │ sync_on_enter │ true      │                                                 │ │
            │                    │ │                                  │ │ file_format   │ plaintext │                                                 │ │
            │                    │ │                                  │ ╰───────────────┴───────────╯                                                 │ │
            │                    │ │                                  │ ╭────────────────┬────────────────────────╮                                   │ │
            │                    │ │ completions                      │ │ case_sensitive │ false                  │                                   │ │
            │                    │ │                                  │ │ quick          │ true                   │                                   │ │
            │                    │ │                                  │ │ partial        │ true                   │                                   │ │
            │                    │ │                                  │ │ algorithm      │ prefix                 │                                   │ │
            │                    │ │                                  │ │                │ ╭─────────────┬──────╮ │                                   │ │
            │                    │ │                                  │ │ external       │ │ enable      │ true │ │                                   │ │
            │                    │ │                                  │ │                │ │ max_results │ 100  │ │                                   │ │
            │                    │ │                                  │ │                │ │ completer   │      │ │                                   │ │
            │                    │ │                                  │ │                │ ╰─────────────┴──────╯ │                                   │ │
            │                    │ │                                  │ ╰────────────────┴────────────────────────╯                                   │ │
            │                    │ │                                  │ ╭────────┬──────╮                                                             │ │
            │                    │ │ filesize                         │ │ metric │ true │                                                             │ │
            │                    │ │                                  │ │ format │ auto │                                                             │ │
            │                    │ │                                  │ ╰────────┴──────╯                                                             │ │
            │                    │ │                                  │ ╭───────────┬────────────╮                                                    │ │
            │                    │ │ cursor_shape                     │ │ emacs     │ line       │                                                    │ │
            │                    │ │                                  │ │ vi_insert │ block      │                                                    │ │
            │                    │ │                                  │ │ vi_normal │ underscore │                                                    │ │
            │                    │ │                                  │ ╰───────────┴────────────╯                                                    │ │
            │                    │ │                                  │ ╭────────────────────────────┬────────────────────╮                           │ │
            │                    │ │ color_config                     │ │ separator                  │ default            │                           │ │
            │                    │ │                                  │ │                            │ ╭──────┬───╮       │                           │ │
            │                    │ │                                  │ │ leading_trailing_space_bg  │ │ attr │ n │       │                           │ │
            │                    │ │                                  │ │                            │ ╰──────┴───╯       │                           │ │
            │                    │ │                                  │ │ header                     │ green_bold         │                           │ │
            │                    │ │                                  │ │ empty                      │ blue               │                           │ │
            │                    │ │                                  │ │ bool                       │                    │                           │ │
            │                    │ │                                  │ │ int                        │ default            │                           │ │
            │                    │ │                                  │ │ filesize                   │                    │                           │ │
            │                    │ │                                  │ │ duration                   │ default            │                           │ │
            │                    │ │                                  │ │ datetime                   │                    │                           │ │
            │                    │ │                                  │ │ range                      │ default            │                           │ │
            │                    │ │                                  │ │ float                      │ default            │                           │ │
            │                    │ │                                  │ │ string                     │ default            │                           │ │
            │                    │ │                                  │ │ nothing                    │ default            │                           │ │
            │                    │ │                                  │ │ binary                     │ default            │                           │ │
            │                    │ │                                  │ │ cell-path                  │ default            │                           │ │
            │                    │ │                                  │ │ row_index                  │ green_bold         │                           │ │
            │                    │ │                                  │ │ record                     │ default            │                           │ │
            │                    │ │                                  │ │ list                       │ default            │                           │ │
            │                    │ │                                  │ │ block                      │ default            │                           │ │
            │                    │ │                                  │ │ hints                      │ dark_gray          │                           │ │
            │                    │ │                                  │ │                            │ ╭────┬───────╮     │                           │ │
            │                    │ │                                  │ │ search_result              │ │ fg │ white │     │                           │ │
            │                    │ │                                  │ │                            │ │ bg │ red   │     │                           │ │
            │                    │ │                                  │ │                            │ ╰────┴───────╯     │                           │ │
            │                    │ │                                  │ │ shape_and                  │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_binary               │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_block                │ blue_bold          │                           │ │
            │                    │ │                                  │ │ shape_bool                 │ light_cyan         │                           │ │
            │                    │ │                                  │ │ shape_custom               │ green              │                           │ │
            │                    │ │                                  │ │ shape_datetime             │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_directory            │ cyan               │                           │ │
            │                    │ │                                  │ │ shape_external             │ cyan               │                           │ │
            │                    │ │                                  │ │ shape_externalarg          │ green_bold         │                           │ │
            │                    │ │                                  │ │ shape_filepath             │ cyan               │                           │ │
            │                    │ │                                  │ │ shape_flag                 │ blue_bold          │                           │ │
            │                    │ │                                  │ │ shape_float                │ purple_bold        │                           │ │
            │                    │ │                                  │ │                            │ ╭──────┬─────────╮ │                           │ │
            │                    │ │                                  │ │ shape_garbage              │ │ fg   │ #FFFFFF │ │                           │ │
            │                    │ │                                  │ │                            │ │ bg   │ #FF0000 │ │                           │ │
            │                    │ │                                  │ │                            │ │ attr │ b       │ │                           │ │
            │                    │ │                                  │ │                            │ ╰──────┴─────────╯ │                           │ │
            │                    │ │                                  │ │ shape_globpattern          │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_int                  │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_internalcall         │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_list                 │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_literal              │ blue               │                           │ │
            │                    │ │                                  │ │                            │ ╭──────┬───╮       │                           │ │
            │                    │ │                                  │ │ shape_matching_brackets    │ │ attr │ u │       │                           │ │
            │                    │ │                                  │ │                            │ ╰──────┴───╯       │                           │ │
            │                    │ │                                  │ │ shape_nothing              │ light_cyan         │                           │ │
            │                    │ │                                  │ │ shape_operator             │ yellow             │                           │ │
            │                    │ │                                  │ │ shape_or                   │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_pipe                 │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_range                │ yellow_bold        │                           │ │
            │                    │ │                                  │ │ shape_record               │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_redirection          │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_signature            │ green_bold         │                           │ │
            │                    │ │                                  │ │ shape_string               │ green              │                           │ │
            │                    │ │                                  │ │ shape_string_interpolation │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_table                │ blue_bold          │                           │ │
            │                    │ │                                  │ │ shape_variable             │ purple             │                           │ │
            │                    │ │                                  │ ╰────────────────────────────┴────────────────────╯                           │ │
            │                    │ │ footer_mode                      │ 25                                                                            │ │
            │                    │ │ float_precision                  │ 2                                                                             │ │
            │                    │ │ use_ansi_coloring                │ true                                                                          │ │
            │                    │ │ edit_mode                        │ emacs                                                                         │ │
            │                    │ │ shell_integration                │ true                                                                          │ │
            │                    │ │ show_banner                      │ true                                                                          │ │
            │                    │ │ render_right_prompt_on_last_line │ false                                                                         │ │
            │                    │ │                                  │ ╭────────────────┬────────────────────╮                                       │ │
            │                    │ │ hooks                            │ │                │ ╭───┬──╮           │                                       │ │
            │                    │ │                                  │ │ pre_prompt     │ │ 0 │  │           │                                       │ │
            │                    │ │                                  │ │                │ ╰───┴──╯           │                                       │ │
            │                    │ │                                  │ │                │ ╭───┬──╮           │                                       │ │
            │                    │ │                                  │ │ pre_execution  │ │ 0 │  │           │                                       │ │
            │                    │ │                                  │ │                │ ╰───┴──╯           │                                       │ │
            │                    │ │                                  │ │                │ ╭─────┬──────────╮ │                                       │ │
            │                    │ │                                  │ │ env_change     │ │     │ ╭───┬──╮ │ │                                       │ │
            │                    │ │                                  │ │                │ │ PWD │ │ 0 │  │ │ │                                       │ │
            │                    │ │                                  │ │                │ │     │ ╰───┴──╯ │ │                                       │ │
            │                    │ │                                  │ │                │ ╰─────┴──────────╯ │                                       │ │
            │                    │ │                                  │ │ display_output │                    │                                       │ │
            │                    │ │                                  │ ╰────────────────┴────────────────────╯                                       │ │
            │                    │ │                                  │ ╭───┬───────────────────────────┬────────────────────────┬────────┬─────╮     │ │
            │                    │ │ menus                            │ │ # │           name            │ only_buffer_difference │ marker │ ... │     │ │
            │                    │ │                                  │ ├───┼───────────────────────────┼────────────────────────┼────────┼─────┤     │ │
            │                    │ │                                  │ │ 0 │ completion_menu           │ false                  │ |      │ ... │     │ │
            │                    │ │                                  │ │ 1 │ history_menu              │ true                   │ ?      │ ... │     │ │
            │                    │ │                                  │ │ 2 │ help_menu                 │ true                   │ ?      │ ... │     │ │
            │                    │ │                                  │ │ 3 │ commands_menu             │ false                  │ #      │ ... │     │ │
            │                    │ │                                  │ │ 4 │ vars_menu                 │ true                   │ #      │ ... │     │ │
            │                    │ │                                  │ │ 5 │ commands_with_description │ true                   │ #      │ ... │     │ │
            │                    │ │                                  │ ╰───┴───────────────────────────┴────────────────────────┴────────┴─────╯     │ │
            │                    │ │                                  │ ╭────┬───────────────────────────┬──────────┬─────────┬────────────────╮      │ │
            │                    │ │ keybindings                      │ │  # │           name            │ modifier │ keycode │      mode      │      │ │
            │                    │ │                                  │ ├────┼───────────────────────────┼──────────┼─────────┼────────────────┤      │ │
            │                    │ │                                  │ │  0 │ completion_menu           │ none     │ tab     │ ╭───┬────────╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs  │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_nor │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ mal    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_ins │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ert    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────────╯ │      │ │
            │                    │ │                                  │ │  1 │ completion_previous       │ shift    │ backtab │ ╭───┬────────╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs  │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_nor │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ mal    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_ins │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ert    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────────╯ │      │ │
            │                    │ │                                  │ │  2 │ history_menu              │ control  │ char_r  │ emacs          │      │ │
            │                    │ │                                  │ │  3 │ next_page                 │ control  │ char_x  │ emacs          │      │ │
            │                    │ │                                  │ │  4 │ undo_or_previous_page     │ control  │ char_z  │ emacs          │      │ │
            │                    │ │                                  │ │  5 │ yank                      │ control  │ char_y  │ emacs          │      │ │
            │                    │ │                                  │ │  6 │ unix-line-discard         │ control  │ char_u  │ ╭───┬────────╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs  │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_nor │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ mal    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_ins │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ert    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────────╯ │      │ │
            │                    │ │                                  │ │  7 │ kill-line                 │ control  │ char_k  │ ╭───┬────────╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs  │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_nor │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ mal    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_ins │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ert    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────────╯ │      │ │
            │                    │ │                                  │ │  8 │ commands_menu             │ control  │ char_t  │ ╭───┬────────╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs  │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_nor │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ mal    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_ins │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ert    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────────╯ │      │ │
            │                    │ │                                  │ │  9 │ vars_menu                 │ alt      │ char_o  │ ╭───┬────────╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs  │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_nor │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ mal    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_ins │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ert    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────────╯ │      │ │
            │                    │ │                                  │ │ 10 │ commands_with_description │ control  │ char_s  │ ╭───┬────────╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs  │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_nor │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ mal    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_ins │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ert    │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────────╯ │      │ │
            │                    │ │                                  │ ╰────┴───────────────────────────┴──────────┴─────────┴────────────────╯      │ │
            │                    │ ╰──────────────────────────────────┴───────────────────────────────────────────────────────────────────────────────╯ │
            ╰────────────────────┴──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯"}
    );
    Ok(())
}

#[test]
fn table_expande_with_no_header_internally_1() -> Result {
    let nu_value = r##"{ "config            ": { "ls": { "use_ls_colors": true, "clickable_links": false }, "rm": { "always_trash": false }, "cd": { "abbreviations": false }, "table": { "mode": "rounded", "index_mode": "always", "trim": { "methodology": "wrapping", "wrapping_try_keep_words": true, "truncating_suffix": "..." } }, "explore": { "help_banner": true, "exit_esc": true, "command_bar_text": "#C4C9C6", "status_bar_background": { "fg": "#1D1F21", "bg": "#C4C9C6" }, "highlight": { "bg": "yellow", "fg": "black" }, "status": {}, "try": {}, "table": { "split_line": "#404040", "cursor": true, "line_index": true, "line_shift": true, "line_head_top": true, "line_head_bottom": true, "show_head": true, "show_index": true }, "config": { "cursor_color": { "bg": "yellow", "fg": "black" } } }, "history": { "max_size": 10000, "sync_on_enter": true, "file_format": "plaintext" }, "completions": { "case_sensitive": false, "quick": true, "partial": true, "algorithm": "prefix", "external": { "enable": true, "max_results": 100, "completer": null } }, "filesize": { "metric": true, "format": "auto" }, "cursor_shape": { "emacs": "line", "vi_insert": "block", "vi_normal": "underscore" }, "color_config": { "separator": "default", "leading_trailing_space_bg": { "attr": "n" }, "header": "green_bold", "empty": "blue", "bool": null, "int": "default", "filesize": null, "duration": "default", "datetime": null, "range": "default", "float": "default", "string": "default", "nothing": "default", "binary": "default", "cell-path": "default", "row_index": "green_bold", "record": "default", "list": "default", "block": "default", "hints": "dark_gray", "search_result": {"fg": "white", "bg": "red"}, "shape_and": "purple_bold", "shape_binary": "purple_bold", "shape_block": "blue_bold", "shape_bool": "light_cyan", "shape_custom": "green", "shape_datetime": "cyan_bold", "shape_directory": "cyan", "shape_external": "cyan", "shape_externalarg": "green_bold", "shape_filepath": "cyan", "shape_flag": "blue_bold", "shape_float": "purple_bold", "shape_garbage": { "fg": "#FFFFFF", "bg": "#FF0000", "attr": "b" }, "shape_globpattern": "cyan_bold", "shape_int": "purple_bold", "shape_internalcall": "cyan_bold", "shape_list": "cyan_bold", "shape_literal": "blue", "shape_matching_brackets": { "attr": "u" }, "shape_nothing": "light_cyan", "shape_operator": "yellow", "shape_or": "purple_bold", "shape_pipe": "purple_bold", "shape_range": "yellow_bold", "shape_record": "cyan_bold", "shape_redirection": "purple_bold", "shape_signature": "green_bold", "shape_string": "green", "shape_string_interpolation": "cyan_bold", "shape_table": "blue_bold", "shape_variable": "purple" }, "footer_mode": "25", "float_precision": 2, "use_ansi_coloring": true, "edit_mode": "emacs", "shell_integration": true, "show_banner": true, "render_right_prompt_on_last_line": false, "hooks": { "pre_prompt": [ null ], "pre_execution": [ null ], "env_change": { "PWD": [ null ] }, "display_output": null }, "menus": [ { "name": "completion_menu", "only_buffer_difference": false, "marker": "| ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "history_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "help_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "commands_menu", "only_buffer_difference": false, "marker": "# ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "vars_menu", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "commands_with_description", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null } ], "keybindings": [ { "name": "completion_menu", "modifier": "none", "keycode": "tab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "send": "menu", "name": "completion_menu" }, { "send": "menunext" } ] } }, { "name": "completion_previous", "modifier": "shift", "keycode": "backtab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menuprevious" } }, { "name": "history_menu", "modifier": "control", "keycode": "char_r", "mode": "emacs", "event": { "send": "menu", "name": "history_menu" } }, { "name": "next_page", "modifier": "control", "keycode": "char_x", "mode": "emacs", "event": { "send": "menupagenext" } }, { "name": "undo_or_previous_page", "modifier": "control", "keycode": "char_z", "mode": "emacs", "event": { "until": [ { "send": "menupageprevious" }, { "edit": "undo" } ] } }, { "name": "yank", "modifier": "control", "keycode": "char_y", "mode": "emacs", "event": { "until": [ { "edit": "pastecutbufferafter" } ] } }, { "name": "unix-line-discard", "modifier": "control", "keycode": "char_u", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cutfromlinestart" } ] } }, { "name": "kill-line", "modifier": "control", "keycode": "char_k", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cuttolineend" } ] } }, { "name": "commands_menu", "modifier": "control", "keycode": "char_t", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_menu" } }, { "name": "vars_menu", "modifier": "alt", "keycode": "char_o", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "vars_menu" } }, { "name": "commands_with_description", "modifier": "control", "keycode": "char_s", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_with_description" } } ] } }"##;
    let mut tester = test();
    let data: Value = tester.run(nu_value.trim())?;
    let actual: String = tester.run_with_data("table --expand --width 136", data)?;
    assert_eq!(
        actual,
        indoc! {"
            ╭────────────────────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
            │                    │ ╭──────────────────────────────────┬──────────────────────────────────────────────────────────────────────────╮ │
            │ config             │ │                                  │ ╭─────────────────┬───────╮                                              │ │
            │                    │ │ ls                               │ │ use_ls_colors   │ true  │                                              │ │
            │                    │ │                                  │ │ clickable_links │ false │                                              │ │
            │                    │ │                                  │ ╰─────────────────┴───────╯                                              │ │
            │                    │ │                                  │ ╭──────────────┬───────╮                                                 │ │
            │                    │ │ rm                               │ │ always_trash │ false │                                                 │ │
            │                    │ │                                  │ ╰──────────────┴───────╯                                                 │ │
            │                    │ │                                  │ ╭───────────────┬───────╮                                                │ │
            │                    │ │ cd                               │ │ abbreviations │ false │                                                │ │
            │                    │ │                                  │ ╰───────────────┴───────╯                                                │ │
            │                    │ │                                  │ ╭────────────┬────────────────────────────────────────╮                  │ │
            │                    │ │ table                            │ │ mode       │ rounded                                │                  │ │
            │                    │ │                                  │ │ index_mode │ always                                 │                  │ │
            │                    │ │                                  │ │            │ ╭─────────────────────────┬──────────╮ │                  │ │
            │                    │ │                                  │ │ trim       │ │ methodology             │ wrapping │ │                  │ │
            │                    │ │                                  │ │            │ │ wrapping_try_keep_words │ true     │ │                  │ │
            │                    │ │                                  │ │            │ │ truncating_suffix       │ ...      │ │                  │ │
            │                    │ │                                  │ │            │ ╰─────────────────────────┴──────────╯ │                  │ │
            │                    │ │                                  │ ╰────────────┴────────────────────────────────────────╯                  │ │
            │                    │ │                                  │ ╭────────────────────────────┬─────────────────────────────────────────╮ │ │
            │                    │ │ explore                          │ │ help_banner                │ true                                    │ │ │
            │                    │ │                                  │ │ exit_esc                   │ true                                    │ │ │
            │                    │ │                                  │ │ command_bar_text           │ #C4C9C6                                 │ │ │
            │                    │ │                                  │ │                            │ ╭────┬─────────╮                        │ │ │
            │                    │ │                                  │ │ status_bar_background      │ │ fg │ #1D1F21 │                        │ │ │
            │                    │ │                                  │ │                            │ │ bg │ #C4C9C6 │                        │ │ │
            │                    │ │                                  │ │                            │ ╰────┴─────────╯                        │ │ │
            │                    │ │                                  │ │                            │ ╭────┬────────╮                         │ │ │
            │                    │ │                                  │ │ highlight                  │ │ bg │ yellow │                         │ │ │
            │                    │ │                                  │ │                            │ │ fg │ black  │                         │ │ │
            │                    │ │                                  │ │                            │ ╰────┴────────╯                         │ │ │
            │                    │ │                                  │ │ status                     │ {record 0 fields}                       │ │ │
            │                    │ │                                  │ │ try                        │ {record 0 fields}                       │ │ │
            │                    │ │                                  │ │                            │ ╭──────────────────┬─────────╮          │ │ │
            │                    │ │                                  │ │ table                      │ │ split_line       │ #404040 │          │ │ │
            │                    │ │                                  │ │                            │ │ cursor           │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ line_index       │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ line_shift       │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ line_head_top    │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ line_head_bottom │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ show_head        │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ show_index       │ true    │          │ │ │
            │                    │ │                                  │ │                            │ ╰──────────────────┴─────────╯          │ │ │
            │                    │ │                                  │ │                            │ ╭──────────────┬─────────────────╮      │ │ │
            │                    │ │                                  │ │ config                     │ │              │ ╭────┬────────╮ │      │ │ │
            │                    │ │                                  │ │                            │ │ cursor_color │ │ bg │ yellow │ │      │ │ │
            │                    │ │                                  │ │                            │ │              │ │ fg │ black  │ │      │ │ │
            │                    │ │                                  │ │                            │ │              │ ╰────┴────────╯ │      │ │ │
            │                    │ │                                  │ │                            │ ╰──────────────┴─────────────────╯      │ │ │
            │                    │ │                                  │ ╰────────────────────────────┴─────────────────────────────────────────╯ │ │
            │                    │ │                                  │ ╭───────────────┬───────────╮                                            │ │
            │                    │ │ history                          │ │ max_size      │ 10000     │                                            │ │
            │                    │ │                                  │ │ sync_on_enter │ true      │                                            │ │
            │                    │ │                                  │ │ file_format   │ plaintext │                                            │ │
            │                    │ │                                  │ ╰───────────────┴───────────╯                                            │ │
            │                    │ │                                  │ ╭────────────────┬────────────────────────╮                              │ │
            │                    │ │ completions                      │ │ case_sensitive │ false                  │                              │ │
            │                    │ │                                  │ │ quick          │ true                   │                              │ │
            │                    │ │                                  │ │ partial        │ true                   │                              │ │
            │                    │ │                                  │ │ algorithm      │ prefix                 │                              │ │
            │                    │ │                                  │ │                │ ╭─────────────┬──────╮ │                              │ │
            │                    │ │                                  │ │ external       │ │ enable      │ true │ │                              │ │
            │                    │ │                                  │ │                │ │ max_results │ 100  │ │                              │ │
            │                    │ │                                  │ │                │ │ completer   │      │ │                              │ │
            │                    │ │                                  │ │                │ ╰─────────────┴──────╯ │                              │ │
            │                    │ │                                  │ ╰────────────────┴────────────────────────╯                              │ │
            │                    │ │                                  │ ╭────────┬──────╮                                                        │ │
            │                    │ │ filesize                         │ │ metric │ true │                                                        │ │
            │                    │ │                                  │ │ format │ auto │                                                        │ │
            │                    │ │                                  │ ╰────────┴──────╯                                                        │ │
            │                    │ │                                  │ ╭───────────┬────────────╮                                               │ │
            │                    │ │ cursor_shape                     │ │ emacs     │ line       │                                               │ │
            │                    │ │                                  │ │ vi_insert │ block      │                                               │ │
            │                    │ │                                  │ │ vi_normal │ underscore │                                               │ │
            │                    │ │                                  │ ╰───────────┴────────────╯                                               │ │
            │                    │ │                                  │ ╭────────────────────────────┬────────────────────╮                      │ │
            │                    │ │ color_config                     │ │ separator                  │ default            │                      │ │
            │                    │ │                                  │ │                            │ ╭──────┬───╮       │                      │ │
            │                    │ │                                  │ │ leading_trailing_space_bg  │ │ attr │ n │       │                      │ │
            │                    │ │                                  │ │                            │ ╰──────┴───╯       │                      │ │
            │                    │ │                                  │ │ header                     │ green_bold         │                      │ │
            │                    │ │                                  │ │ empty                      │ blue               │                      │ │
            │                    │ │                                  │ │ bool                       │                    │                      │ │
            │                    │ │                                  │ │ int                        │ default            │                      │ │
            │                    │ │                                  │ │ filesize                   │                    │                      │ │
            │                    │ │                                  │ │ duration                   │ default            │                      │ │
            │                    │ │                                  │ │ datetime                   │                    │                      │ │
            │                    │ │                                  │ │ range                      │ default            │                      │ │
            │                    │ │                                  │ │ float                      │ default            │                      │ │
            │                    │ │                                  │ │ string                     │ default            │                      │ │
            │                    │ │                                  │ │ nothing                    │ default            │                      │ │
            │                    │ │                                  │ │ binary                     │ default            │                      │ │
            │                    │ │                                  │ │ cell-path                  │ default            │                      │ │
            │                    │ │                                  │ │ row_index                  │ green_bold         │                      │ │
            │                    │ │                                  │ │ record                     │ default            │                      │ │
            │                    │ │                                  │ │ list                       │ default            │                      │ │
            │                    │ │                                  │ │ block                      │ default            │                      │ │
            │                    │ │                                  │ │ hints                      │ dark_gray          │                      │ │
            │                    │ │                                  │ │                            │ ╭────┬───────╮     │                      │ │
            │                    │ │                                  │ │ search_result              │ │ fg │ white │     │                      │ │
            │                    │ │                                  │ │                            │ │ bg │ red   │     │                      │ │
            │                    │ │                                  │ │                            │ ╰────┴───────╯     │                      │ │
            │                    │ │                                  │ │ shape_and                  │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_binary               │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_block                │ blue_bold          │                      │ │
            │                    │ │                                  │ │ shape_bool                 │ light_cyan         │                      │ │
            │                    │ │                                  │ │ shape_custom               │ green              │                      │ │
            │                    │ │                                  │ │ shape_datetime             │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_directory            │ cyan               │                      │ │
            │                    │ │                                  │ │ shape_external             │ cyan               │                      │ │
            │                    │ │                                  │ │ shape_externalarg          │ green_bold         │                      │ │
            │                    │ │                                  │ │ shape_filepath             │ cyan               │                      │ │
            │                    │ │                                  │ │ shape_flag                 │ blue_bold          │                      │ │
            │                    │ │                                  │ │ shape_float                │ purple_bold        │                      │ │
            │                    │ │                                  │ │                            │ ╭──────┬─────────╮ │                      │ │
            │                    │ │                                  │ │ shape_garbage              │ │ fg   │ #FFFFFF │ │                      │ │
            │                    │ │                                  │ │                            │ │ bg   │ #FF0000 │ │                      │ │
            │                    │ │                                  │ │                            │ │ attr │ b       │ │                      │ │
            │                    │ │                                  │ │                            │ ╰──────┴─────────╯ │                      │ │
            │                    │ │                                  │ │ shape_globpattern          │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_int                  │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_internalcall         │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_list                 │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_literal              │ blue               │                      │ │
            │                    │ │                                  │ │                            │ ╭──────┬───╮       │                      │ │
            │                    │ │                                  │ │ shape_matching_brackets    │ │ attr │ u │       │                      │ │
            │                    │ │                                  │ │                            │ ╰──────┴───╯       │                      │ │
            │                    │ │                                  │ │ shape_nothing              │ light_cyan         │                      │ │
            │                    │ │                                  │ │ shape_operator             │ yellow             │                      │ │
            │                    │ │                                  │ │ shape_or                   │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_pipe                 │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_range                │ yellow_bold        │                      │ │
            │                    │ │                                  │ │ shape_record               │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_redirection          │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_signature            │ green_bold         │                      │ │
            │                    │ │                                  │ │ shape_string               │ green              │                      │ │
            │                    │ │                                  │ │ shape_string_interpolation │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_table                │ blue_bold          │                      │ │
            │                    │ │                                  │ │ shape_variable             │ purple             │                      │ │
            │                    │ │                                  │ ╰────────────────────────────┴────────────────────╯                      │ │
            │                    │ │ footer_mode                      │ 25                                                                       │ │
            │                    │ │ float_precision                  │ 2                                                                        │ │
            │                    │ │ use_ansi_coloring                │ true                                                                     │ │
            │                    │ │ edit_mode                        │ emacs                                                                    │ │
            │                    │ │ shell_integration                │ true                                                                     │ │
            │                    │ │ show_banner                      │ true                                                                     │ │
            │                    │ │ render_right_prompt_on_last_line │ false                                                                    │ │
            │                    │ │                                  │ ╭────────────────┬────────────────────╮                                  │ │
            │                    │ │ hooks                            │ │                │ ╭───┬──╮           │                                  │ │
            │                    │ │                                  │ │ pre_prompt     │ │ 0 │  │           │                                  │ │
            │                    │ │                                  │ │                │ ╰───┴──╯           │                                  │ │
            │                    │ │                                  │ │                │ ╭───┬──╮           │                                  │ │
            │                    │ │                                  │ │ pre_execution  │ │ 0 │  │           │                                  │ │
            │                    │ │                                  │ │                │ ╰───┴──╯           │                                  │ │
            │                    │ │                                  │ │                │ ╭─────┬──────────╮ │                                  │ │
            │                    │ │                                  │ │ env_change     │ │     │ ╭───┬──╮ │ │                                  │ │
            │                    │ │                                  │ │                │ │ PWD │ │ 0 │  │ │ │                                  │ │
            │                    │ │                                  │ │                │ │     │ ╰───┴──╯ │ │                                  │ │
            │                    │ │                                  │ │                │ ╰─────┴──────────╯ │                                  │ │
            │                    │ │                                  │ │ display_output │                    │                                  │ │
            │                    │ │                                  │ ╰────────────────┴────────────────────╯                                  │ │
            │                    │ │                                  │ ╭───┬───────────────────────────┬────────────────────────┬───────┬─────╮ │ │
            │                    │ │ menus                            │ │ # │           name            │ only_buffer_difference │ marke │ ... │ │ │
            │                    │ │                                  │ │   │                           │                        │ r     │     │ │ │
            │                    │ │                                  │ ├───┼───────────────────────────┼────────────────────────┼───────┼─────┤ │ │
            │                    │ │                                  │ │ 0 │ completion_menu           │ false                  │ |     │ ... │ │ │
            │                    │ │                                  │ │ 1 │ history_menu              │ true                   │ ?     │ ... │ │ │
            │                    │ │                                  │ │ 2 │ help_menu                 │ true                   │ ?     │ ... │ │ │
            │                    │ │                                  │ │ 3 │ commands_menu             │ false                  │ #     │ ... │ │ │
            │                    │ │                                  │ │ 4 │ vars_menu                 │ true                   │ #     │ ... │ │ │
            │                    │ │                                  │ │ 5 │ commands_with_description │ true                   │ #     │ ... │ │ │
            │                    │ │                                  │ ╰───┴───────────────────────────┴────────────────────────┴───────┴─────╯ │ │
            │                    │ │                                  │ ╭────┬───────────────────────────┬──────────┬─────────┬───────────╮      │ │
            │                    │ │ keybindings                      │ │  # │           name            │ modifier │ keycode │   mode    │      │ │
            │                    │ │                                  │ ├────┼───────────────────────────┼──────────┼─────────┼───────────┤      │ │
            │                    │ │                                  │ │  0 │ completion_menu           │ none     │ tab     │ ╭───┬───╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ c │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ o │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴───╯ │      │ │
            │                    │ │                                  │ │  1 │ completion_previous       │ shift    │ backtab │ ╭───┬───╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ c │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ o │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴───╯ │      │ │
            │                    │ │                                  │ │  2 │ history_menu              │ control  │ char_r  │ emacs     │      │ │
            │                    │ │                                  │ │  3 │ next_page                 │ control  │ char_x  │ emacs     │      │ │
            │                    │ │                                  │ │  4 │ undo_or_previous_page     │ control  │ char_z  │ emacs     │      │ │
            │                    │ │                                  │ │  5 │ yank                      │ control  │ char_y  │ emacs     │      │ │
            │                    │ │                                  │ │  6 │ unix-line-discard         │ control  │ char_u  │ ╭───┬───╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ c │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ o │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴───╯ │      │ │
            │                    │ │                                  │ │  7 │ kill-line                 │ control  │ char_k  │ ╭───┬───╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ c │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ o │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴───╯ │      │ │
            │                    │ │                                  │ │  8 │ commands_menu             │ control  │ char_t  │ ╭───┬───╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ c │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ o │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴───╯ │      │ │
            │                    │ │                                  │ │  9 │ vars_menu                 │ alt      │ char_o  │ ╭───┬───╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ c │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ o │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴───╯ │      │ │
            │                    │ │                                  │ │ 10 │ commands_with_description │ control  │ char_s  │ ╭───┬───╮ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ c │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ o │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ m │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ a │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ v │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _ │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ i │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ n │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ e │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ r │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t │ │      │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴───╯ │      │ │
            │                    │ │                                  │ ╰────┴───────────────────────────┴──────────┴─────────┴───────────╯      │ │
            │                    │ ╰──────────────────────────────────┴──────────────────────────────────────────────────────────────────────────╯ │
            ╰────────────────────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯"}
    );
    Ok(())
}

#[test]
fn big_table_expanded_with_padding_0() -> Result {
    let nu_value = r##"{ "config            ": { "ls": { "use_ls_colors": true, "clickable_links": false }, "rm": { "always_trash": false }, "cd": { "abbreviations": false }, "table": { "mode": "rounded", "index_mode": "always", "trim": { "methodology": "wrapping", "wrapping_try_keep_words": true, "truncating_suffix": "..." } }, "explore": { "help_banner": true, "exit_esc": true, "command_bar_text": "#C4C9C6", "status_bar_background": { "fg": "#1D1F21", "bg": "#C4C9C6" }, "highlight": { "bg": "yellow", "fg": "black" }, "status": {}, "try": {}, "table": { "split_line": "#404040", "cursor": true, "line_index": true, "line_shift": true, "line_head_top": true, "line_head_bottom": true, "show_head": true, "show_index": true }, "config": { "cursor_color": { "bg": "yellow", "fg": "black" } } }, "history": { "max_size": 10000, "sync_on_enter": true, "file_format": "plaintext" }, "completions": { "case_sensitive": false, "quick": true, "partial": true, "algorithm": "prefix", "external": { "enable": true, "max_results": 100, "completer": null } }, "filesize": { "metric": true, "format": "auto" }, "cursor_shape": { "emacs": "line", "vi_insert": "block", "vi_normal": "underscore" }, "color_config": { "separator": "default", "leading_trailing_space_bg": { "attr": "n" }, "header": "green_bold", "empty": "blue", "bool": null, "int": "default", "filesize": null, "duration": "default", "datetime": null, "range": "default", "float": "default", "string": "default", "nothing": "default", "binary": "default", "cell-path": "default", "row_index": "green_bold", "record": "default", "list": "default", "block": "default", "hints": "dark_gray", "search_result": {"fg": "white", "bg": "red"}, "shape_and": "purple_bold", "shape_binary": "purple_bold", "shape_block": "blue_bold", "shape_bool": "light_cyan", "shape_custom": "green", "shape_datetime": "cyan_bold", "shape_directory": "cyan", "shape_external": "cyan", "shape_externalarg": "green_bold", "shape_filepath": "cyan", "shape_flag": "blue_bold", "shape_float": "purple_bold", "shape_garbage": { "fg": "#FFFFFF", "bg": "#FF0000", "attr": "b" }, "shape_globpattern": "cyan_bold", "shape_int": "purple_bold", "shape_internalcall": "cyan_bold", "shape_list": "cyan_bold", "shape_literal": "blue", "shape_matching_brackets": { "attr": "u" }, "shape_nothing": "light_cyan", "shape_operator": "yellow", "shape_or": "purple_bold", "shape_pipe": "purple_bold", "shape_range": "yellow_bold", "shape_record": "cyan_bold", "shape_redirection": "purple_bold", "shape_signature": "green_bold", "shape_string": "green", "shape_string_interpolation": "cyan_bold", "shape_table": "blue_bold", "shape_variable": "purple" }, "footer_mode": "25", "float_precision": 2, "use_ansi_coloring": true, "edit_mode": "emacs", "shell_integration": true, "show_banner": true, "render_right_prompt_on_last_line": false, "hooks": { "pre_prompt": [ null ], "pre_execution": [ null ], "env_change": { "PWD": [ null ] }, "display_output": null }, "menus": [ { "name": "completion_menu", "only_buffer_difference": false, "marker": "| ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "history_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "help_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "commands_menu", "only_buffer_difference": false, "marker": "# ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "vars_menu", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "commands_with_description", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null } ], "keybindings": [ { "name": "completion_menu", "modifier": "none", "keycode": "tab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "send": "menu", "name": "completion_menu" }, { "send": "menunext" } ] } }, { "name": "completion_previous", "modifier": "shift", "keycode": "backtab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menuprevious" } }, { "name": "history_menu", "modifier": "control", "keycode": "char_r", "mode": "emacs", "event": { "send": "menu", "name": "history_menu" } }, { "name": "next_page", "modifier": "control", "keycode": "char_x", "mode": "emacs", "event": { "send": "menupagenext" } }, { "name": "undo_or_previous_page", "modifier": "control", "keycode": "char_z", "mode": "emacs", "event": { "until": [ { "send": "menupageprevious" }, { "edit": "undo" } ] } }, { "name": "yank", "modifier": "control", "keycode": "char_y", "mode": "emacs", "event": { "until": [ { "edit": "pastecutbufferafter" } ] } }, { "name": "unix-line-discard", "modifier": "control", "keycode": "char_u", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cutfromlinestart" } ] } }, { "name": "kill-line", "modifier": "control", "keycode": "char_k", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cuttolineend" } ] } }, { "name": "commands_menu", "modifier": "control", "keycode": "char_t", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_menu" } }, { "name": "vars_menu", "modifier": "alt", "keycode": "char_o", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "vars_menu" } }, { "name": "commands_with_description", "modifier": "control", "keycode": "char_s", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_with_description" } } ] } }"##;
    let mut tester = test();
    let data: Value = tester.run(nu_value.trim())?;
    let actual: String = tester.run_with_data(
        "
            let data = $in
            $env.config.table.padding = { left: 2, right: 3 }
            $data
            | table --expand --width 141
        ",
        data,
    )?;
    assert_eq!(
        actual,
        indoc! {"
            ╭───────────────────────┬───────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
            │                       │  ╭─────────────────────────────────────┬──────────────────────────────────────────────────────────────────────╮   │
            │  config               │  │                                     │  ╭────────────────────┬──────────╮                                   │   │
            │                       │  │  ls                                 │  │  use_ls_colors     │  true    │                                   │   │
            │                       │  │                                     │  │  clickable_links   │  false   │                                   │   │
            │                       │  │                                     │  ╰────────────────────┴──────────╯                                   │   │
            │                       │  │                                     │  ╭─────────────────┬──────────╮                                      │   │
            │                       │  │  rm                                 │  │  always_trash   │  false   │                                      │   │
            │                       │  │                                     │  ╰─────────────────┴──────────╯                                      │   │
            │                       │  │                                     │  ╭──────────────────┬──────────╮                                     │   │
            │                       │  │  cd                                 │  │  abbreviations   │  false   │                                     │   │
            │                       │  │                                     │  ╰──────────────────┴──────────╯                                     │   │
            │                       │  │                                     │  ╭───────────────┬───────────────────────────────────────────────╮   │   │
            │                       │  │  table                              │  │  mode         │  rounded                                      │   │   │
            │                       │  │                                     │  │  index_mode   │  always                                       │   │   │
            │                       │  │                                     │  │               │  ╭────────────────────────────┬───────────╮   │   │   │
            │                       │  │                                     │  │  trim         │  │  methodology               │  wrappi   │   │   │   │
            │                       │  │                                     │  │               │  │                            │  ng       │   │   │   │
            │                       │  │                                     │  │               │  │  wrapping_try_keep_words   │  true     │   │   │   │
            │                       │  │                                     │  │               │  │  truncating_suffix         │  ...      │   │   │   │
            │                       │  │                                     │  │               │  ╰────────────────────────────┴───────────╯   │   │   │
            │                       │  │                                     │  ╰───────────────┴───────────────────────────────────────────────╯   │   │
            │                       │  │                                     │  ╭──────────────────────────┬────────────────────────────────────╮   │   │
            │                       │  │  explore                            │  │  help_banner             │  true                              │   │   │
            │                       │  │                                     │  │  exit_esc                │  true                              │   │   │
            │                       │  │                                     │  │  command_bar_text        │  #C4C9C6                           │   │   │
            │                       │  │                                     │  │                          │  ╭───────┬────────────╮            │   │   │
            │                       │  │                                     │  │  status_bar_background   │  │  fg   │  #1D1F21   │            │   │   │
            │                       │  │                                     │  │                          │  │  bg   │  #C4C9C6   │            │   │   │
            │                       │  │                                     │  │                          │  ╰───────┴────────────╯            │   │   │
            │                       │  │                                     │  │                          │  ╭───────┬───────────╮             │   │   │
            │                       │  │                                     │  │  highlight               │  │  bg   │  yellow   │             │   │   │
            │                       │  │                                     │  │                          │  │  fg   │  black    │             │   │   │
            │                       │  │                                     │  │                          │  ╰───────┴───────────╯             │   │   │
            │                       │  │                                     │  │  status                  │  {record 0 fields}                 │   │   │
            │                       │  │                                     │  │  try                     │  {record 0 fields}                 │   │   │
            │                       │  │                                     │  │  table                   │  {record 8 fields}                 │   │   │
            │                       │  │                                     │  │                          │  ╭─────────────────┬───────────╮   │   │   │
            │                       │  │                                     │  │  config                  │  │  cursor_color   │  {recor   │   │   │   │
            │                       │  │                                     │  │                          │  │                 │  d 2 fi   │   │   │   │
            │                       │  │                                     │  │                          │  │                 │  elds}    │   │   │   │
            │                       │  │                                     │  │                          │  ╰─────────────────┴───────────╯   │   │   │
            │                       │  │                                     │  ╰──────────────────────────┴────────────────────────────────────╯   │   │
            │                       │  │                                     │  ╭──────────────────┬──────────────╮                                 │   │
            │                       │  │  history                            │  │  max_size        │  10000       │                                 │   │
            │                       │  │                                     │  │  sync_on_enter   │  true        │                                 │   │
            │                       │  │                                     │  │  file_format     │  plaintext   │                                 │   │
            │                       │  │                                     │  ╰──────────────────┴──────────────╯                                 │   │
            │                       │  │                                     │  ╭────────────────────────┬──────────────────────────────────────╮   │   │
            │                       │  │  completions                        │  │  case_sensitive        │  false                               │   │   │
            │                       │  │                                     │  │  quick                 │  true                                │   │   │
            │                       │  │                                     │  │  partial               │  true                                │   │   │
            │                       │  │                                     │  │  algorithm             │  prefix                              │   │   │
            │                       │  │                                     │  │                        │  ╭────────────────┬─────────╮        │   │   │
            │                       │  │                                     │  │  external              │  │  enable        │  true   │        │   │   │
            │                       │  │                                     │  │                        │  │  max_results   │  100    │        │   │   │
            │                       │  │                                     │  │                        │  │  completer     │         │        │   │   │
            │                       │  │                                     │  │                        │  ╰────────────────┴─────────╯        │   │   │
            │                       │  │                                     │  ╰────────────────────────┴──────────────────────────────────────╯   │   │
            │                       │  │                                     │  ╭───────────┬─────────╮                                             │   │
            │                       │  │  filesize                           │  │  metric   │  true   │                                             │   │
            │                       │  │                                     │  │  format   │  auto   │                                             │   │
            │                       │  │                                     │  ╰───────────┴─────────╯                                             │   │
            │                       │  │                                     │  ╭──────────────┬───────────────╮                                    │   │
            │                       │  │  cursor_shape                       │  │  emacs       │  line         │                                    │   │
            │                       │  │                                     │  │  vi_insert   │  block        │                                    │   │
            │                       │  │                                     │  │  vi_normal   │  underscore   │                                    │   │
            │                       │  │                                     │  ╰──────────────┴───────────────╯                                    │   │
            │                       │  │                                     │  ╭───────────────────────────────┬───────────────────────────────╮   │   │
            │                       │  │  color_config                       │  │  separator                    │  default                      │   │   │
            │                       │  │                                     │  │                               │  ╭─────────┬──────╮           │   │   │
            │                       │  │                                     │  │  leading_trailing_space_bg    │  │  attr   │  n   │           │   │   │
            │                       │  │                                     │  │                               │  ╰─────────┴──────╯           │   │   │
            │                       │  │                                     │  │  header                       │  green_bold                   │   │   │
            │                       │  │                                     │  │  empty                        │  blue                         │   │   │
            │                       │  │                                     │  │  bool                         │                               │   │   │
            │                       │  │                                     │  │  int                          │  default                      │   │   │
            │                       │  │                                     │  │  filesize                     │                               │   │   │
            │                       │  │                                     │  │  duration                     │  default                      │   │   │
            │                       │  │                                     │  │  datetime                     │                               │   │   │
            │                       │  │                                     │  │  range                        │  default                      │   │   │
            │                       │  │                                     │  │  float                        │  default                      │   │   │
            │                       │  │                                     │  │  string                       │  default                      │   │   │
            │                       │  │                                     │  │  nothing                      │  default                      │   │   │
            │                       │  │                                     │  │  binary                       │  default                      │   │   │
            │                       │  │                                     │  │  cell-path                    │  default                      │   │   │
            │                       │  │                                     │  │  row_index                    │  green_bold                   │   │   │
            │                       │  │                                     │  │  record                       │  default                      │   │   │
            │                       │  │                                     │  │  list                         │  default                      │   │   │
            │                       │  │                                     │  │  block                        │  default                      │   │   │
            │                       │  │                                     │  │  hints                        │  dark_gray                    │   │   │
            │                       │  │                                     │  │                               │  ╭───────┬──────────╮         │   │   │
            │                       │  │                                     │  │  search_result                │  │  fg   │  white   │         │   │   │
            │                       │  │                                     │  │                               │  │  bg   │  red     │         │   │   │
            │                       │  │                                     │  │                               │  ╰───────┴──────────╯         │   │   │
            │                       │  │                                     │  │  shape_and                    │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_binary                 │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_block                  │  blue_bold                    │   │   │
            │                       │  │                                     │  │  shape_bool                   │  light_cyan                   │   │   │
            │                       │  │                                     │  │  shape_custom                 │  green                        │   │   │
            │                       │  │                                     │  │  shape_datetime               │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_directory              │  cyan                         │   │   │
            │                       │  │                                     │  │  shape_external               │  cyan                         │   │   │
            │                       │  │                                     │  │  shape_externalarg            │  green_bold                   │   │   │
            │                       │  │                                     │  │  shape_filepath               │  cyan                         │   │   │
            │                       │  │                                     │  │  shape_flag                   │  blue_bold                    │   │   │
            │                       │  │                                     │  │  shape_float                  │  purple_bold                  │   │   │
            │                       │  │                                     │  │                               │  ╭──────────┬─────────────╮   │   │   │
            │                       │  │                                     │  │  shape_garbage                │  │  fg      │  #FFFFFF    │   │   │   │
            │                       │  │                                     │  │                               │  │  bg      │  #FF0000    │   │   │   │
            │                       │  │                                     │  │                               │  │  attr    │  b          │   │   │   │
            │                       │  │                                     │  │                               │  ╰──────────┴─────────────╯   │   │   │
            │                       │  │                                     │  │  shape_globpattern            │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_int                    │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_internalcall           │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_list                   │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_literal                │  blue                         │   │   │
            │                       │  │                                     │  │                               │  ╭─────────┬──────╮           │   │   │
            │                       │  │                                     │  │  shape_matching_brackets      │  │  attr   │  u   │           │   │   │
            │                       │  │                                     │  │                               │  ╰─────────┴──────╯           │   │   │
            │                       │  │                                     │  │  shape_nothing                │  light_cyan                   │   │   │
            │                       │  │                                     │  │  shape_operator               │  yellow                       │   │   │
            │                       │  │                                     │  │  shape_or                     │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_pipe                   │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_range                  │  yellow_bold                  │   │   │
            │                       │  │                                     │  │  shape_record                 │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_redirection            │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_signature              │  green_bold                   │   │   │
            │                       │  │                                     │  │  shape_string                 │  green                        │   │   │
            │                       │  │                                     │  │  shape_string_interpolation   │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_table                  │  blue_bold                    │   │   │
            │                       │  │                                     │  │  shape_variable               │  purple                       │   │   │
            │                       │  │                                     │  ╰───────────────────────────────┴───────────────────────────────╯   │   │
            │                       │  │  footer_mode                        │  25                                                                  │   │
            │                       │  │  float_precision                    │  2                                                                   │   │
            │                       │  │  use_ansi_coloring                  │  true                                                                │   │
            │                       │  │  edit_mode                          │  emacs                                                               │   │
            │                       │  │  shell_integration                  │  true                                                                │   │
            │                       │  │  show_banner                        │  true                                                                │   │
            │                       │  │  render_right_prompt_on_last_line   │  false                                                               │   │
            │                       │  │                                     │  ╭───────────────────────┬───────────────────────────────────────╮   │   │
            │                       │  │  hooks                              │  │                       │  ╭──────┬─────╮                       │   │   │
            │                       │  │                                     │  │  pre_prompt           │  │  0   │     │                       │   │   │
            │                       │  │                                     │  │                       │  ╰──────┴─────╯                       │   │   │
            │                       │  │                                     │  │                       │  ╭──────┬─────╮                       │   │   │
            │                       │  │                                     │  │  pre_execution        │  │  0   │     │                       │   │   │
            │                       │  │                                     │  │                       │  ╰──────┴─────╯                       │   │   │
            │                       │  │                                     │  │                       │  ╭────────┬───────────────────╮       │   │   │
            │                       │  │                                     │  │  env_change           │  │        │  ╭──────┬─────╮   │       │   │   │
            │                       │  │                                     │  │                       │  │  PWD   │  │  0   │     │   │       │   │   │
            │                       │  │                                     │  │                       │  │        │  ╰──────┴─────╯   │       │   │   │
            │                       │  │                                     │  │                       │  ╰────────┴───────────────────╯       │   │   │
            │                       │  │                                     │  │  display_output       │                                       │   │   │
            │                       │  │                                     │  ╰───────────────────────┴───────────────────────────────────────╯   │   │
            │                       │  │                                     │  ╭──────┬──────────────────────────────┬────────────────┬────────╮   │   │
            │                       │  │  menus                              │  │  #   │            name              │  only_buffer   │  ...   │   │   │
            │                       │  │                                     │  │      │                              │  _difference   │        │   │   │
            │                       │  │                                     │  ├──────┼──────────────────────────────┼────────────────┼────────┤   │   │
            │                       │  │                                     │  │  0   │  completion_menu             │  false         │  ...   │   │   │
            │                       │  │                                     │  │  1   │  history_menu                │  true          │  ...   │   │   │
            │                       │  │                                     │  │  2   │  help_menu                   │  true          │  ...   │   │   │
            │                       │  │                                     │  │  3   │  commands_menu               │  false         │  ...   │   │   │
            │                       │  │                                     │  │  4   │  vars_menu                   │  true          │  ...   │   │   │
            │                       │  │                                     │  │  5   │  commands_with_description   │  true          │  ...   │   │   │
            │                       │  │                                     │  ╰──────┴──────────────────────────────┴────────────────┴────────╯   │   │
            │                       │  │                                     │  ╭───────┬──────────────────────────────┬─────────────┬────────╮     │   │
            │                       │  │  keybindings                        │  │   #   │            name              │  modifier   │  ...   │     │   │
            │                       │  │                                     │  ├───────┼──────────────────────────────┼─────────────┼────────┤     │   │
            │                       │  │                                     │  │   0   │  completion_menu             │  none       │  ...   │     │   │
            │                       │  │                                     │  │   1   │  completion_previous         │  shift      │  ...   │     │   │
            │                       │  │                                     │  │   2   │  history_menu                │  control    │  ...   │     │   │
            │                       │  │                                     │  │   3   │  next_page                   │  control    │  ...   │     │   │
            │                       │  │                                     │  │   4   │  undo_or_previous_page       │  control    │  ...   │     │   │
            │                       │  │                                     │  │   5   │  yank                        │  control    │  ...   │     │   │
            │                       │  │                                     │  │   6   │  unix-line-discard           │  control    │  ...   │     │   │
            │                       │  │                                     │  │   7   │  kill-line                   │  control    │  ...   │     │   │
            │                       │  │                                     │  │   8   │  commands_menu               │  control    │  ...   │     │   │
            │                       │  │                                     │  │   9   │  vars_menu                   │  alt        │  ...   │     │   │
            │                       │  │                                     │  │  10   │  commands_with_description   │  control    │  ...   │     │   │
            │                       │  │                                     │  ╰───────┴──────────────────────────────┴─────────────┴────────╯     │   │
            │                       │  ╰─────────────────────────────────────┴──────────────────────────────────────────────────────────────────────╯   │
            ╰───────────────────────┴───────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯"}
    );
    Ok(())
}

#[test]
fn test_collapse_big_0() -> Result {
    Playground::setup("test_expand_big_0", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
            [package]
            authors = ["The Nushell Project Developers"]
            default-run = "nu"
            description = "A new type of shell"
            documentation = "https://www.nushell.sh/book/"
            edition = "2024"
            exclude = ["images"]
            homepage = "https://www.nushell.sh"
            license = "MIT"
            name = "nu"
            repository = "https://github.com/nushell/nushell"
            rust-version = "1.60"
            version = "0.74.1"


            # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html


            [package.metadata.binstall]
            pkg-url = "{ repo }/releases/download/{ version }/{ name }-{ version }-{ target }.{ archive-format }"
            pkg-fmt = "tgz"


            [package.metadata.binstall.overrides.x86_64-pc-windows-msvc]
            pkg-fmt = "zip"


            [workspace]
            members = [
                "crates/nu-cli",
                "crates/nu-engine",
                "crates/nu-parser",
                "crates/nu-system",
                "crates/nu-command",
                "crates/nu-protocol",
                "crates/nu-plugin",
                "crates/nu_plugin_inc",
                "crates/nu_plugin_gstat",
                "crates/nu_plugin_example",
                "crates/nu_plugin_query",
                "crates/nu_plugin_custom_values",
                "crates/nu-utils",
            ]


            [dependencies]
            chrono = { version = "0.4.23", features = ["serde"] }
            crossterm = "0.24.0"
            ctrlc = "3.2.1"
            log = "0.4"
            miette = { version = "5.5.0", features = ["fancy-no-backtrace"] }
            nu-ansi-term = "0.46.0"
            nu-cli = { path = "./crates/nu-cli", version = "0.74.1" }
            nu-engine = { path = "./crates/nu-engine", version = "0.74.1" }
            reedline = { version = "0.14.0", features = ["bashisms", "sqlite"] }


            rayon = "1.6.1"
            is_executable = "1.0.1"
            simplelog = "0.12.0"
            time = "0.3.12"


            [target.'cfg(not(target_os = "windows"))'.dependencies]
            # Our dependencies don't use OpenSSL on Windows
            openssl = { version = "0.10.38", features = ["vendored"], optional = true }
            signal-hook = { version = "0.3.14", default-features = false }




            [target.'cfg(windows)'.build-dependencies]
            winres = "0.1"


            [target.'cfg(target_family = "unix")'.dependencies]
            nix = { version = "0.25", default-features = false, features = ["signal", "process", "fs", "term"] }
            atty = "0.2"


            [dev-dependencies]
            nu-test-support = { path = "./crates/nu-test-support", version = "0.74.1" }
            tempfile = "3.2.0"
            assert_cmd = "2.0.2"
            criterion = "0.4"
            pretty_assertions = "1.0.0"
            serial_test = "0.10.0"
            hamcrest2 = "0.3.0"
            rstest = { version = "0.15.0", default-features = false }
            itertools = "0.10.3"


            [features]
            plugin = [
                "nu-plugin",
                "nu-cli/plugin",
                "nu-parser/plugin",
                "nu-command/plugin",
                "nu-protocol/plugin",
                "nu-engine/plugin",
            ]
            # extra used to be more useful but now it's the same as default. Leaving it in for backcompat with existing build scripts
            extra = ["default"]
            default = ["plugin", "which-support", "trash-support", "sqlite"]
            stable = ["default"]
            wasi = []


            # Enable to statically link OpenSSL; otherwise the system version will be used. Not enabled by default because it takes a while to build
            static-link-openssl = ["dep:openssl"]


            # Stable (Default)
            which-support = ["nu-command/which-support"]
            trash-support = ["nu-command/trash-support"]


            # Main nu binary
            [[bin]]
            name = "nu"
            path = "src/main.rs"


            # To use a development version of a dependency please use a global override here
            # changing versions in each sub-crate of the workspace is tedious
            [patch.crates-io]
            reedline = { git = "https://github.com/nushell/reedline.git", branch = "main" }


            # Criterion benchmarking setup
            # Run all benchmarks with `cargo bench`
            # Run individual benchmarks like `cargo bench -- <regex>` e.g. `cargo bench -- parse`
            [[bench]]
            name = "benchmarks"
            harness = false
            "#,
        )]);
        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --width=80 --collapse")?;
        let expected = indoc! {r#"
            ╭──────────────────┬───────────────┬───────────────────────────────────────────╮
            │ package          │ authors       │ The Nushell Project Developers            │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ default-run   │ nu                                        │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ description   │ A new type of shell                       │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ documentation │ https://www.nushell.sh/book/              │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ edition       │ 2024                                      │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ exclude       │ images                                    │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ homepage      │ https://www.nushell.sh                    │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ license       │ MIT                                       │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ name          │ nu                                        │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ repository    │ https://github.com/nushell/nushell        │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ rust-version  │ 1.60                                      │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ version       │ 0.74.1                                    │
            │                  ├───────────────┼──────────┬───────────┬────────────────────┤
            │                  │ metadata      │ binstall │ pkg-url   │ { repo }/releases/ │
            │                  │               │          │           │ download/{ v       │
            │                  │               │          │           │ ersion }/{ name }- │
            │                  │               │          │           │ { version }-       │
            │                  │               │          │           │ { target }.{ archi │
            │                  │               │          │           │ ve-format }        │
            │                  │               │          ├───────────┼────────────────────┤
            │                  │               │          │ pkg-fmt   │ tgz                │
            │                  │               │          ├───────────┼────────────────────┤
            │                  │               │          │ overrides │ ...                │
            ├──────────────────┼─────────┬─────┴──────────┴───────────┴────────────────────┤
            │ workspace        │ members │ crates/nu-cli                                   │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-engine                                │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-parser                                │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-system                                │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-command                               │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-protocol                              │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-plugin                                │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_inc                            │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_gstat                          │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_example                        │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_query                          │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_custom_values                  │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-utils                                 │
            ├──────────────────┼─────────┴─────┬──────────┬────────────────────────────────┤
            │ dependencies     │ chrono        │ version  │ 0.4.23                         │
            │                  │               ├──────────┼────────────────────────────────┤
            │                  │               │ features │ serde                          │
            │                  ├───────────────┼──────────┴────────────────────────────────┤
            │                  │ crossterm     │ 0.24.0                                    │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ ctrlc         │ 3.2.1                                     │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ log           │ 0.4                                       │
            │                  ├───────────────┼──────────┬────────────────────────────────┤
            │                  │ miette        │ version  │ 5.5.0                          │
            │                  │               ├──────────┼────────────────────────────────┤
            │                  │               │ features │ fancy-no-backtrace             │
            │                  ├───────────────┼──────────┴────────────────────────────────┤
            │                  │ nu-ansi-term  │ 0.46.0                                    │
            │                  ├───────────────┼─────────┬─────────────────────────────────┤
            │                  │ nu-cli        │ path    │ ./crates/nu-cli                 │
            │                  │               ├─────────┼─────────────────────────────────┤
            │                  │               │ version │ 0.74.1                          │
            │                  ├───────────────┼─────────┼─────────────────────────────────┤
            │                  │ nu-engine     │ path    │ ./crates/nu-engine              │
            │                  │               ├─────────┼─────────────────────────────────┤
            │                  │               │ version │ 0.74.1                          │
            │                  ├───────────────┼─────────┴┬────────────────────────────────┤
            │                  │ reedline      │ version  │ 0.14.0                         │
            │                  │               ├──────────┼────────────────────────────────┤
            │                  │               │ features │ bashisms                       │
            │                  │               │          ├────────────────────────────────┤
            │                  │               │          │ sqlite                         │
            │                  ├───────────────┼──────────┴────────────────────────────────┤
            │                  │ rayon         │ 1.6.1                                     │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ is_executable │ 1.0.1                                     │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ simplelog     │ 0.12.0                                    │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ time          │ 0.3.12                                    │
            ├──────────────────┼───────────────┴─────────────────┬──────────────┬──────────┤
            │ target           │ cfg(not(target_os = "windows")) │ dependencies │ ...      │
            │                  │                                 │              ├──────────┤
            │                  │                                 │              │ ...      │
            │                  ├─────────────────────────────────┼──────────────┴──────────┤
            │                  │ cfg(windows)                    │ ...                     │
            │                  ├─────────────────────────────────┼──────────────┬──────────┤
            │                  │ cfg(target_family = "unix")     │ dependencies │ ...      │
            │                  │                                 │              ├──────────┤
            │                  │                                 │              │ ...      │
            ├──────────────────┼───────────────────┬─────────┬───┴──────────────┴──────────┤
            │ dev-dependencies │ nu-test-support   │ path    │ ./crates/nu-test-support    │
            │                  │                   ├─────────┼─────────────────────────────┤
            │                  │                   │ version │ 0.74.1                      │
            │                  ├───────────────────┼─────────┴─────────────────────────────┤
            │                  │ tempfile          │ 3.2.0                                 │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ assert_cmd        │ 2.0.2                                 │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ criterion         │ 0.4                                   │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ pretty_assertions │ 1.0.0                                 │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ serial_test       │ 0.10.0                                │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ hamcrest2         │ 0.3.0                                 │
            │                  ├───────────────────┼──────────────────┬────────────────────┤
            │                  │ rstest            │ version          │ 0.15.0             │
            │                  │                   ├──────────────────┼────────────────────┤
            │                  │                   │ default-features │ false              │
            │                  ├───────────────────┼──────────────────┴────────────────────┤
            │                  │ itertools         │ 0.10.3                                │
            ├──────────────────┼───────────────────┴─┬─────────────────────────────────────┤
            │ features         │ plugin              │ nu-plugin                           │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-cli/plugin                       │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-parser/plugin                    │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-command/plugin                   │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-protocol/plugin                  │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-engine/plugin                    │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ extra               │ default                             │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ default             │ plugin                              │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ which-support                       │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ trash-support                       │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ sqlite                              │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ stable              │ default                             │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ wasi                │                                     │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ static-link-openssl │ dep:openssl                         │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ which-support       │ nu-command/which-support            │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ trash-support       │ nu-command/trash-support            │
            ├──────────────────┼──────┬──────────────┴─────────────────────────────────────┤
            │ bin              │ name │ path                                               │
            │                  ├──────┼────────────────────────────────────────────────────┤
            │                  │ nu   │ src/main.rs                                        │
            ├──────────────────┼──────┴────┬──────────┬────────┬───────────────────────────┤
            │ patch            │ crates-io │ reedline │ git    │ https://github.com/nushel │
            │                  │           │          │        │ l/reedline.git            │
            │                  │           │          ├────────┼───────────────────────────┤
            │                  │           │          │ branch │ main                      │
            ├──────────────────┼───────────┴┬─────────┴────────┴───────────────────────────┤
            │ bench            │ name       │ harness                                      │
            │                  ├────────────┼──────────────────────────────────────────────┤
            │                  │ benchmarks │ false                                        │
            ╰──────────────────┴────────────┴──────────────────────────────────────────────╯"#};
        assert_eq!(actual, expected);
        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --collapse --width=160")?;
        let expected = indoc! {r#"
            ╭──────────────────┬───────────────┬──────────────────────────────────────────────────────────────────────────╮
            │ package          │ authors       │ The Nushell Project Developers                                           │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ default-run   │ nu                                                                       │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ description   │ A new type of shell                                                      │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ documentation │ https://www.nushell.sh/book/                                             │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ edition       │ 2024                                                                     │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ exclude       │ images                                                                   │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ homepage      │ https://www.nushell.sh                                                   │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ license       │ MIT                                                                      │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ name          │ nu                                                                       │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ repository    │ https://github.com/nushell/nushell                                       │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ rust-version  │ 1.60                                                                     │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ version       │ 0.74.1                                                                   │
            │                  ├───────────────┼──────────┬───────────┬───────────────────────────────────────────────────┤
            │                  │ metadata      │ binstall │ pkg-url   │ { repo }/releases/download/{ v                    │
            │                  │               │          │           │ ersion }/{ name }-{ version }-                    │
            │                  │               │          │           │ { target }.{ archive-format }                     │
            │                  │               │          ├───────────┼───────────────────────────────────────────────────┤
            │                  │               │          │ pkg-fmt   │ tgz                                               │
            │                  │               │          ├───────────┼────────────────────────┬─────────┬────────────────┤
            │                  │               │          │ overrides │ x86_64-pc-windows-msvc │ pkg-fmt │ zip            │
            ├──────────────────┼─────────┬─────┴──────────┴───────────┴────────────────────────┴─────────┴────────────────┤
            │ workspace        │ members │ crates/nu-cli                                                                  │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-engine                                                               │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-parser                                                               │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-system                                                               │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-command                                                              │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-protocol                                                             │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-plugin                                                               │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_inc                                                           │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_gstat                                                         │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_example                                                       │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_query                                                         │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_custom_values                                                 │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-utils                                                                │
            ├──────────────────┼─────────┴─────┬──────────┬───────────────────────────────────────────────────────────────┤
            │ dependencies     │ chrono        │ version  │ 0.4.23                                                        │
            │                  │               ├──────────┼───────────────────────────────────────────────────────────────┤
            │                  │               │ features │ serde                                                         │
            │                  ├───────────────┼──────────┴───────────────────────────────────────────────────────────────┤
            │                  │ crossterm     │ 0.24.0                                                                   │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ ctrlc         │ 3.2.1                                                                    │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ log           │ 0.4                                                                      │
            │                  ├───────────────┼──────────┬───────────────────────────────────────────────────────────────┤
            │                  │ miette        │ version  │ 5.5.0                                                         │
            │                  │               ├──────────┼───────────────────────────────────────────────────────────────┤
            │                  │               │ features │ fancy-no-backtrace                                            │
            │                  ├───────────────┼──────────┴───────────────────────────────────────────────────────────────┤
            │                  │ nu-ansi-term  │ 0.46.0                                                                   │
            │                  ├───────────────┼─────────┬────────────────────────────────────────────────────────────────┤
            │                  │ nu-cli        │ path    │ ./crates/nu-cli                                                │
            │                  │               ├─────────┼────────────────────────────────────────────────────────────────┤
            │                  │               │ version │ 0.74.1                                                         │
            │                  ├───────────────┼─────────┼────────────────────────────────────────────────────────────────┤
            │                  │ nu-engine     │ path    │ ./crates/nu-engine                                             │
            │                  │               ├─────────┼────────────────────────────────────────────────────────────────┤
            │                  │               │ version │ 0.74.1                                                         │
            │                  ├───────────────┼─────────┴┬───────────────────────────────────────────────────────────────┤
            │                  │ reedline      │ version  │ 0.14.0                                                        │
            │                  │               ├──────────┼───────────────────────────────────────────────────────────────┤
            │                  │               │ features │ bashisms                                                      │
            │                  │               │          ├───────────────────────────────────────────────────────────────┤
            │                  │               │          │ sqlite                                                        │
            │                  ├───────────────┼──────────┴───────────────────────────────────────────────────────────────┤
            │                  │ rayon         │ 1.6.1                                                                    │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ is_executable │ 1.0.1                                                                    │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ simplelog     │ 0.12.0                                                                   │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ time          │ 0.3.12                                                                   │
            ├──────────────────┼───────────────┴─────────────────┬──────────────┬─────────────┬──────────┬────────────────┤
            │ target           │ cfg(not(target_os = "windows")) │ dependencies │ openssl     │ version  │ 0.10.38        │
            │                  │                                 │              │             ├──────────┼────────────────┤
            │                  │                                 │              │             │ features │ vendored       │
            │                  │                                 │              │             ├──────────┼────────────────┤
            │                  │                                 │              │             │ optional │ true           │
            │                  │                                 │              ├─────────────┼──────────┴───────┬────────┤
            │                  │                                 │              │ signal-hook │ version          │ 0.3.14 │
            │                  │                                 │              │             ├──────────────────┼────────┤
            │                  │                                 │              │             │ default-features │ false  │
            │                  ├─────────────────────────────────┼──────────────┴─────┬───────┴┬─────────────────┴────────┤
            │                  │ cfg(windows)                    │ build-dependencies │ winres │ 0.1                      │
            │                  ├─────────────────────────────────┼──────────────┬─────┴┬───────┴──────────┬───────────────┤
            │                  │ cfg(target_family = "unix")     │ dependencies │ nix  │ version          │ 0.25          │
            │                  │                                 │              │      ├──────────────────┼───────────────┤
            │                  │                                 │              │      │ default-features │ false         │
            │                  │                                 │              │      ├──────────────────┼───────────────┤
            │                  │                                 │              │      │ features         │ signal        │
            │                  │                                 │              │      │                  ├───────────────┤
            │                  │                                 │              │      │                  │ process       │
            │                  │                                 │              │      │                  ├───────────────┤
            │                  │                                 │              │      │                  │ fs            │
            │                  │                                 │              │      │                  ├───────────────┤
            │                  │                                 │              │      │                  │ term          │
            │                  │                                 │              ├──────┼──────────────────┴───────────────┤
            │                  │                                 │              │ atty │ 0.2                              │
            ├──────────────────┼───────────────────┬─────────┬───┴──────────────┴──────┴──────────────────────────────────┤
            │ dev-dependencies │ nu-test-support   │ path    │ ./crates/nu-test-support                                   │
            │                  │                   ├─────────┼────────────────────────────────────────────────────────────┤
            │                  │                   │ version │ 0.74.1                                                     │
            │                  ├───────────────────┼─────────┴────────────────────────────────────────────────────────────┤
            │                  │ tempfile          │ 3.2.0                                                                │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ assert_cmd        │ 2.0.2                                                                │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ criterion         │ 0.4                                                                  │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ pretty_assertions │ 1.0.0                                                                │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ serial_test       │ 0.10.0                                                               │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ hamcrest2         │ 0.3.0                                                                │
            │                  ├───────────────────┼──────────────────┬───────────────────────────────────────────────────┤
            │                  │ rstest            │ version          │ 0.15.0                                            │
            │                  │                   ├──────────────────┼───────────────────────────────────────────────────┤
            │                  │                   │ default-features │ false                                             │
            │                  ├───────────────────┼──────────────────┴───────────────────────────────────────────────────┤
            │                  │ itertools         │ 0.10.3                                                               │
            ├──────────────────┼───────────────────┴─┬────────────────────────────────────────────────────────────────────┤
            │ features         │ plugin              │ nu-plugin                                                          │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-cli/plugin                                                      │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-parser/plugin                                                   │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-command/plugin                                                  │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-protocol/plugin                                                 │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-engine/plugin                                                   │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ extra               │ default                                                            │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ default             │ plugin                                                             │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ which-support                                                      │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ trash-support                                                      │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ sqlite                                                             │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ stable              │ default                                                            │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ wasi                │                                                                    │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ static-link-openssl │ dep:openssl                                                        │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ which-support       │ nu-command/which-support                                           │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ trash-support       │ nu-command/trash-support                                           │
            ├──────────────────┼──────┬──────────────┴────────────────────────────────────────────────────────────────────┤
            │ bin              │ name │ path                                                                              │
            │                  ├──────┼───────────────────────────────────────────────────────────────────────────────────┤
            │                  │ nu   │ src/main.rs                                                                       │
            ├──────────────────┼──────┴────┬──────────┬────────┬──────────────────────────────────────────────────────────┤
            │ patch            │ crates-io │ reedline │ git    │ https://github.com/nushell/reedline.git                  │
            │                  │           │          ├────────┼──────────────────────────────────────────────────────────┤
            │                  │           │          │ branch │ main                                                     │
            ├──────────────────┼───────────┴┬─────────┴────────┴──────────────────────────────────────────────────────────┤
            │ bench            │ name       │ harness                                                                     │
            │                  ├────────────┼─────────────────────────────────────────────────────────────────────────────┤
            │                  │ benchmarks │ false                                                                       │
            ╰──────────────────┴────────────┴─────────────────────────────────────────────────────────────────────────────╯"#};
        assert_eq!(actual, expected);
        Ok(())
    })
}

#[test]
fn table_expand_index_offset() -> Result {
    let actual: String = test().run("1..1002 | table --width=80 --expand")?;
    let suffix = indoc! {"
        ╭──────┬──────╮
        │ 1000 │ 1001 │
        │ 1001 │ 1002 │
        ╰──────┴──────╯
    "};
    let expected_suffix = actual.strip_suffix(suffix);
    assert!(expected_suffix.is_some(), "{actual:?}");
    Ok(())
}

#[test]
fn table_index_offset() -> Result {
    let actual: String = test().run("1..1002 | table --width=80")?;
    let suffix = indoc! {"
        ╭──────┬──────╮
        │ 1000 │ 1001 │
        │ 1001 │ 1002 │
        ╰──────┴──────╯
    "};
    let expected_suffix = actual.strip_suffix(suffix);
    assert!(expected_suffix.is_some(), "{actual:?}");
    Ok(())
}
