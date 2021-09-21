use crate::{
    evaluate::{lang, scope::Scope},
    EvaluationContext,
};
use indexmap::IndexMap;
use nu_data::config::path::{default_history_path, history_path};
use nu_errors::ShellError;
use nu_protocol::{Dictionary, ShellTypeName, Signature, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::{Spanned, Tag};

pub fn nu(scope: &Scope, ctx: &EvaluationContext) -> Result<Value, ShellError> {
    let env = &scope.get_env_vars();
    let tag = Tag::unknown();

    let mut nu_dict = TaggedDictBuilder::new(&tag);

    let mut dict = TaggedDictBuilder::new(&tag);

    for v in env {
        if v.0 != "PATH" && v.0 != "Path" {
            dict.insert_untagged(v.0, UntaggedValue::string(v.1));
        }
    }

    nu_dict.insert_value("env", dict.into_value());

    nu_dict.insert_value(
        "history-path",
        UntaggedValue::filepath(default_history_path()).into_value(&tag),
    );

    if let Some(global_cfg) = &ctx.configs().lock().global_config {
        nu_dict.insert_value(
            "config",
            UntaggedValue::row(global_cfg.vars.clone()).into_value(&tag),
        );

        nu_dict.insert_value(
            "config-path",
            UntaggedValue::filepath(global_cfg.file_path.clone()).into_value(&tag),
        );

        // overwrite hist-path if present
        if let Some(hist_path) = history_path(global_cfg) {
            nu_dict.insert_value(
                "history-path",
                UntaggedValue::filepath(hist_path).into_value(&tag),
            );
        }
    }

    // A note about environment variables:
    //
    // Environment variables in Unix platforms are case-sensitive. On Windows, case-sensitivity is context-dependent.
    // In cmd.exe, running `SET` will show you the list of environment variables and their names will be mixed case.
    // In PowerShell, running `Get-ChildItem Env:` will show you a list of environment variables, and they will match
    // the case in the environment variable section of the user configuration
    //
    // Rust currently returns the DOS-style, all-uppercase environment variables on Windows (as of 1.52) when running
    // std::env::vars(), rather than the case-sensitive Environment.GetEnvironmentVariables() of .NET that PowerShell
    // uses.
    //
    // For now, we work around the discrepency as best we can by merging the two into what is shown to the user as the
    // 'path' column of `$nu`
    let mut table = vec![];
    for v in env {
        if v.0 == "PATH" || v.0 == "Path" {
            for path in std::env::split_paths(&v.1) {
                table.push(UntaggedValue::filepath(path).into_value(&tag));
            }
        }
    }
    nu_dict.insert_value("path", UntaggedValue::table(&table).into_value(&tag));

    let path = std::env::current_dir()?;
    nu_dict.insert_value("cwd", UntaggedValue::filepath(path).into_value(&tag));

    if let Some(home) = crate::filesystem::filesystem_shell::homedir_if_possible() {
        nu_dict.insert_value("home-dir", UntaggedValue::filepath(home).into_value(&tag));
    }

    let temp = std::env::temp_dir();
    nu_dict.insert_value("temp-dir", UntaggedValue::filepath(temp).into_value(&tag));

    #[cfg(feature = "rustyline-support")]
    {
        let keybinding_path = nu_data::keybinding::keybinding_path()?;
        nu_dict.insert_value(
            "keybinding-path",
            UntaggedValue::filepath(keybinding_path).into_value(&tag),
        );
    }

    let cmd_info = lang::Lang::query_commands(scope);
    match cmd_info {
        Ok(cmds) => nu_dict.insert_value("lang", UntaggedValue::table(&cmds).into_value(&tag)),
        Err(_) => nu_dict.insert_value("lang", UntaggedValue::string("no commands found")),
    }

    Ok(nu_dict.into_value())
}

pub fn scope(
    aliases: &IndexMap<String, Vec<Spanned<String>>>,
    commands: &IndexMap<String, Signature>,
    variables: &IndexMap<String, Value>,
) -> Result<Value, ShellError> {
    let tag = Tag::unknown();

    let mut scope_dict = TaggedDictBuilder::new(&tag);

    let mut aliases_dict = TaggedDictBuilder::new(&tag);
    for v in aliases {
        let values = v.1.clone();
        let mut vec = Vec::new();

        for k in &values {
            vec.push(k.to_string());
        }

        let alias = vec.join(" ");

        aliases_dict.insert_untagged(v.0, UntaggedValue::string(alias));
    }

    let mut commands_dict = TaggedDictBuilder::new(&tag);
    for (name, signature) in commands {
        commands_dict.insert_untagged(name, UntaggedValue::string(&signature.allowed().join(" ")))
    }

    let var_list: Vec<Value> = variables
        .iter()
        .map(|var| {
            let mut entries: IndexMap<String, Value> = IndexMap::new();
            let name = var.0.trim_start_matches('$');
            entries.insert(
                "name".to_string(),
                UntaggedValue::string(name).into_value(&tag),
            );
            entries.insert(
                "value".to_string(),
                UntaggedValue::string(var.1.convert_to_string()).into_value(&tag),
            );
            entries.insert(
                "type".to_string(),
                UntaggedValue::string(ShellTypeName::type_name(&var.1)).into_value(&tag),
            );
            UntaggedValue::Row(Dictionary { entries }).into_value(&tag)
        })
        .collect();

    scope_dict.insert_value("aliases", aliases_dict.into_value());

    scope_dict.insert_value("commands", commands_dict.into_value());

    scope_dict.insert_value("variables", UntaggedValue::Table(var_list).into_value(&tag));

    Ok(scope_dict.into_value())
}
