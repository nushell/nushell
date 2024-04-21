use super::{PluginCacheFile, PluginCacheItem, PluginCacheItemData};
use crate::{
    Category, PluginExample, PluginSignature, ShellError, Signature, SyntaxShape, Type, Value,
};
use pretty_assertions::assert_eq;
use std::io::Cursor;

fn foo_plugin() -> PluginCacheItem {
    PluginCacheItem {
        name: "foo".into(),
        filename: "/path/to/nu_plugin_foo".into(),
        shell: None,
        data: PluginCacheItemData::Valid {
            commands: vec![PluginSignature {
                sig: Signature::new("foo")
                    .input_output_type(Type::Int, Type::List(Box::new(Type::Int)))
                    .category(Category::Experimental),
                examples: vec![PluginExample {
                    example: "16 | foo".into(),
                    description: "powers of two up to 16".into(),
                    result: Some(Value::test_list(vec![
                        Value::test_int(2),
                        Value::test_int(4),
                        Value::test_int(8),
                        Value::test_int(16),
                    ])),
                }],
            }],
        },
    }
}

fn bar_plugin() -> PluginCacheItem {
    PluginCacheItem {
        name: "bar".into(),
        filename: "/path/to/nu_plugin_bar".into(),
        shell: None,
        data: PluginCacheItemData::Valid {
            commands: vec![PluginSignature {
                sig: Signature::new("bar")
                    .usage("overwrites files with random data")
                    .switch("force", "ignore errors", Some('f'))
                    .required(
                        "path",
                        SyntaxShape::Filepath,
                        "file to overwrite with random data",
                    )
                    .category(Category::Experimental),
                examples: vec![],
            }],
        },
    }
}

#[test]
fn roundtrip() -> Result<(), ShellError> {
    let mut plugin_cache_file = PluginCacheFile {
        nushell_version: env!("CARGO_PKG_VERSION").to_owned(),
        plugins: vec![foo_plugin(), bar_plugin()],
    };

    let mut output = vec![];

    plugin_cache_file.write_to(&mut output, None)?;

    let read_file = PluginCacheFile::read_from(Cursor::new(&output[..]), None)?;

    assert_eq!(plugin_cache_file, read_file);

    Ok(())
}

#[test]
fn roundtrip_invalid() -> Result<(), ShellError> {
    let mut plugin_cache_file = PluginCacheFile {
        nushell_version: env!("CARGO_PKG_VERSION").to_owned(),
        plugins: vec![PluginCacheItem {
            name: "invalid".into(),
            filename: "/path/to/nu_plugin_invalid".into(),
            shell: None,
            data: PluginCacheItemData::Invalid,
        }],
    };

    let mut output = vec![];

    plugin_cache_file.write_to(&mut output, None)?;

    let read_file = PluginCacheFile::read_from(Cursor::new(&output[..]), None)?;

    assert_eq!(plugin_cache_file, read_file);

    Ok(())
}

#[test]
fn upsert_new() {
    let mut file = PluginCacheFile::new();

    file.plugins.push(foo_plugin());

    file.upsert_plugin(bar_plugin());

    assert_eq!(2, file.plugins.len());
}

#[test]
fn upsert_replace() {
    let mut file = PluginCacheFile::new();

    file.plugins.push(foo_plugin());

    let mut mutated_foo = foo_plugin();
    mutated_foo.shell = Some("/bin/sh".into());

    file.upsert_plugin(mutated_foo);

    assert_eq!(1, file.plugins.len());
    assert_eq!(Some("/bin/sh".into()), file.plugins[0].shell);
}
