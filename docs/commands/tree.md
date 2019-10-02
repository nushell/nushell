# tree

Use `tree` to display directory paths and (optionally) files in each subdirectory. 

When you use the TREE command each directory name is displayed along with the names of any subdirectories within it. The display will be in a format like the summary below. (Different versions of DOS may display the data in a slightly different format.)

## Syntax

tree (directory name)(folder name)*
*- Optional

## Examples

```shell
> tree
├── nushell
    ├── assets
    │   ├── syntaxes.bin
    │   └── themes.bin
    ├── Cargo.lock
    ├── Cargo.toml
    ├── CODE_OF_CONDUCT.md
    ├── debian
    │   ├── changelog
    │   ├── compat
    │   ├── control
    │   ├── copyright
    │   ├── install
    │   ├── postinst
    │   ├── postrm
    │   ├── rules
    │   └── source
    │       └── format
    ├── docker
    │   ├── docker-compose.package.yml
    │   ├── Dockerfile
    │   ├── Dockerfile.nu-base
    │   ├── Package.Dockerfile
    │   ├── Package.glibc-busybox.Dockerfile
    │   ├── Package.glibc-distroless.Dockerfile
    │   └── packaging
    │       ├── Dockerfile.ubuntu-bionic
    │       └── README.md
    ├── docs
    │   ├── commands
    │   │   ├── cd.md
    │   │   ├── date.md
    │   │   ├── echo.md
    │   │   ├── help.md
    │   │   ├── README.md
    │   │   └── tree.md
    │   ├── docker.md
    │   └── philosophy.md
    ├── images
    │   ├── nushell-autocomplete3.gif
    │   ├── nushell-autocomplete4.gif
    │   └── nushell-autocomplete.gif
    ├── LICENSE
    ├── Makefile.toml
    ├── README.md
    ├── rustfmt.toml
    ├── rust-toolchain
    ├── src
    │   ├── cli.rs
    │   ├── commands
    │   │   ├── args.rs
    │   │   ├── autoview.rs
    │   │   ├── cd.rs
    │   │   ├── classified.rs
    │   │   ├── clip.rs
    │   │   ├── command.rs
    │   │   ├── config.rs
    │   │   ├── cp.rs
    │   │   ├── date.rs
    │   │   ├── debug.rs
    │   │   ├── echo.rs
    │   │   ├── enter.rs
    │   │   ├── env.rs
    │   │   ├── exit.rs
    │   │   ├── fetch.rs
    │   │   ├── first.rs
    │   │   ├── format.rs
    │   │   ├── from_bson.rs
    │   │   ├── from_csv.rs
    │   │   ├── from_ini.rs
    │   │   ├── from_json.rs
    │   │   ├── from_sqlite.rs
    │   │   ├── from_toml.rs
    │   │   ├── from_tsv.rs
    │   │   ├── from_url.rs
    │   │   ├── from_xml.rs
    │   │   ├── from_yaml.rs
    │   │   ├── get.rs
    │   │   ├── help.rs
    │   │   ├── last.rs
    │   │   ├── lines.rs
    │   │   ├── ls.rs
    │   │   ├── macros.rs
    │   │   ├── mkdir.rs
    │   │   ├── mv.rs
    │   │   ├── next.rs
    │   │   ├── nth.rs
    │   │   ├── open.rs
    │   │   ├── pick.rs
    │   │   ├── pivot.rs
    │   │   ├── plugin.rs
    │   │   ├── post.rs
    │   │   ├── prev.rs
    │   │   ├── pwd.rs
    │   │   ├── reject.rs
    │   │   ├── reverse.rs
    │   │   ├── rm.rs
    │   │   ├── save.rs
    │   │   ├── shells.rs
    │   │   ├── size.rs
    │   │   ├── skip_while.rs
    │   │   ├── sort_by.rs
    │   │   ├── split_column.rs
    │   │   ├── split_row.rs
    │   │   ├── table.rs
    │   │   ├── tags.rs
    │   │   ├── to_bson.rs
    │   │   ├── to_csv.rs
    │   │   ├── to_json.rs
    │   │   ├── to_sqlite.rs
    │   │   ├── to_toml.rs
    │   │   ├── to_tsv.rs
    │   │   ├── to_url.rs
    │   │   ├── to_yaml.rs
    │   │   ├── trim.rs
    │   │   ├── version.rs
    │   │   ├── where_.rs
    │   │   └── which_.rs
    │   ├── commands.rs
    │   ├── context.rs
    │   ├── data
    │   │   ├── base.rs
    │   │   ├── command.rs
    │   │   ├── config.rs
    │   │   ├── dict.rs
    │   │   ├── files.rs
    │   │   ├── into.rs
    │   │   ├── meta.rs
    │   │   ├── operators.rs
    │   │   ├── process.rs
    │   │   └── types.rs
    │   ├── data.rs
    │   ├── env
    │   │   └── host.rs
    │   ├── env.rs
    │   ├── errors.rs
    │   ├── evaluate
    │   │   ├── evaluator.rs
    │   │   └── mod.rs
    │   ├── format
    │   │   ├── entries.rs
    │   │   ├── generic.rs
    │   │   ├── list.rs
    │   │   └── table.rs
    │   ├── format.rs
    │   ├── fuzzysearch.rs
    │   ├── git.rs
    │   ├── lib.rs
    │   ├── main.rs
    │   ├── parser
    │   │   ├── deserializer.rs
    │   │   ├── hir
    │   │   │   ├── baseline_parse.rs
    │   │   │   ├── baseline_parse_tokens.rs
    │   │   │   ├── binary.rs
    │   │   │   ├── external_command.rs
    │   │   │   ├── named.rs
    │   │   │   └── path.rs
    │   │   ├── hir.rs
    │   │   ├── parse
    │   │   │   ├── call_node.rs
    │   │   │   ├── files.rs
    │   │   │   ├── flag.rs
    │   │   │   ├── operator.rs
    │   │   │   ├── parser.rs
    │   │   │   ├── pipeline.rs
    │   │   │   ├── text.rs
    │   │   │   ├── tokens.rs
    │   │   │   ├── token_tree_builder.rs
    │   │   │   ├── token_tree.rs
    │   │   │   ├── unit.rs
    │   │   │   └── util.rs
    │   │   ├── parse_command.rs
    │   │   ├── parse.rs
    │   │   └── registry.rs
    │   ├── parser.rs
    │   ├── plugin.rs
    │   ├── plugins
    │   │   ├── add.rs
    │   │   ├── binaryview.rs
    │   │   ├── docker.rs
    │   │   ├── edit.rs
    │   │   ├── embed.rs
    │   │   ├── inc.rs
    │   │   ├── ps.rs
    │   │   ├── skip.rs
    │   │   ├── str.rs
    │   │   ├── sum.rs
    │   │   ├── sys.rs
    │   │   ├── textview.rs
    │   │   └── tree.rs
    │   ├── prelude.rs
    │   ├── shell
    │   │   ├── completer.rs
    │   │   ├── filesystem_shell.rs
    │   │   ├── helper.rs
    │   │   ├── help_shell.rs
    │   │   ├── shell_manager.rs
    │   │   ├── shell.rs
    │   │   └── value_shell.rs
    │   ├── shell.rs
    │   ├── stream.rs
    │   ├── traits.rs
    │   └── utils.rs
    └── tests
        ├── command_cd_tests.rs
        ├── command_config_test.rs
        ├── command_cp_tests.rs
        ├── command_enter_test.rs
        ├── command_ls_tests.rs
        ├── command_mkdir_tests.rs
        ├── command_mv_tests.rs
        ├── command_open_tests.rs
        ├── command_rm_tests.rs
        ├── commands_test.rs
        ├── external_tests.rs
        ├── filter_inc_tests.rs
        ├── filters_test.rs
        ├── filter_str_tests.rs
        ├── fixtures
        │   ├── formats
        │   │   ├── appveyor.yml
        │   │   ├── caco3_plastics.csv
        │   │   ├── caco3_plastics.tsv
        │   │   ├── cargo_sample.toml
        │   │   ├── jonathan.xml
        │   │   ├── sample.bson
        │   │   ├── sample.db
        │   │   ├── sample.ini
        │   │   ├── sample.url
        │   │   ├── sgml_description.json
        │   │   └── utf16.ini
        │   └── nuplayground
        ├── helpers
        │   └── mod.rs
        └── tests.rs
```