use nu_engine::value_shell::ValueShell;
use nu_protocol::ColumnPath;
use nu_source::SpannedItem;

use super::matchers::Matcher;
use crate::{Completer, CompletionContext, Suggestion};
use std::path::{Path, PathBuf};

fn build_path(head: &str, members: &Path, entry: &str) -> String {
    let mut full_path = head.to_string();
    full_path.push_str(
        &members
            .join(entry)
            .display()
            .to_string()
            .replace(std::path::MAIN_SEPARATOR, "."),
    );
    full_path
}

fn collect_entries(value_fs: &ValueShell, head: &str, path: &Path) -> Vec<String> {
    value_fs
        .members_under(&path)
        .iter()
        .flat_map(|entry| {
            entry
                .row_entries()
                .map(|(entry_name, _)| build_path(&head, &path, entry_name))
        })
        .collect()
}

pub struct VariableCompleter;

impl<Context> Completer<Context> for VariableCompleter
where
    Context: CompletionContext,
{
    fn complete(&self, ctx: &Context, partial: &str, matcher: &dyn Matcher) -> Vec<Suggestion> {
        let registry = ctx.variable_registry();
        let variables_available = registry.variables();
        let partial_column_path = ColumnPath::with_head(&partial.to_string().spanned_unknown());

        partial_column_path
            .map(|(head, members)| {
                variables_available
                    .iter()
                    .filter(|candidate| matcher.matches(&head, candidate))
                    .into_iter()
                    .filter_map(|candidate| {
                        if !partial.ends_with('.') && members.is_empty() {
                            Some(vec![candidate.to_string()])
                        } else {
                            let value = registry.get_variable(&candidate[..].spanned_unknown());
                            let path = PathBuf::from(members.path());

                            value.map(|candidate| {
                                let fs = ValueShell::new(candidate);

                                fs.find(&path)
                                    .map(|fs| collect_entries(fs, &head, &path))
                                    .or_else(|| {
                                        path.parent().map(|parent| {
                                            fs.find(parent)
                                                .map(|fs| collect_entries(fs, &head, &parent))
                                                .unwrap_or_default()
                                        })
                                    })
                                    .unwrap_or_default()
                            })
                        }
                    })
                    .flatten()
                    .filter_map(|candidate| {
                        if matcher.matches(&partial, &candidate) {
                            Some(Suggestion::new(&candidate, &candidate))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::{Completer, Suggestion as S, VariableCompleter};
    use crate::matchers::case_insensitive::Matcher as CaseInsensitiveMatcher;

    use indexmap::IndexMap;
    use nu_engine::{
        evaluation_context::EngineState, ConfigHolder, EvaluationContext, FakeHost, Host, Scope,
        ShellManager,
    };
    use nu_protocol::{SignatureRegistry, VariableRegistry};
    use parking_lot::Mutex;
    use std::ffi::OsString;
    use std::sync::{atomic::AtomicBool, Arc};

    struct CompletionContext<'a>(&'a EvaluationContext);

    impl<'a> crate::CompletionContext for CompletionContext<'a> {
        fn signature_registry(&self) -> &dyn SignatureRegistry {
            &self.0.scope
        }

        fn source(&self) -> &nu_engine::EvaluationContext {
            &self.0
        }

        fn scope(&self) -> &dyn nu_parser::ParserScope {
            &self.0.scope
        }

        fn variable_registry(&self) -> &dyn VariableRegistry {
            self.0
        }
    }

    fn create_context_with_host(host: Box<dyn Host>) -> EvaluationContext {
        let scope = Scope::new();
        let env_vars = host.vars().iter().cloned().collect::<IndexMap<_, _>>();
        scope.add_env(env_vars);

        EvaluationContext {
            scope,
            engine_state: Arc::new(EngineState {
                host: Arc::new(parking_lot::Mutex::new(host)),
                current_errors: Arc::new(Mutex::new(vec![])),
                ctrl_c: Arc::new(AtomicBool::new(false)),
                configs: Arc::new(Mutex::new(ConfigHolder::new())),
                shell_manager: ShellManager::basic(),
                windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
            }),
        }
    }

    fn set_envs(host: &mut FakeHost, values: Vec<(&str, &str)>) {
        values.iter().for_each(|(key, value)| {
            host.env_set(OsString::from(key), OsString::from(value));
        });
    }

    #[test]
    fn structure() {
        let mut host = nu_engine::FakeHost::new();
        set_envs(&mut host, vec![("COMPLETER", "VARIABLE"), ("SHELL", "NU")]);
        let context = create_context_with_host(Box::new(host));

        assert_eq!(
            VariableCompleter {}.complete(
                &CompletionContext(&context),
                "$nu.env.",
                &CaseInsensitiveMatcher
            ),
            vec![
                S::new("$nu.env.COMPLETER", "$nu.env.COMPLETER"),
                S::new("$nu.env.SHELL", "$nu.env.SHELL")
            ]
        );

        assert_eq!(
            VariableCompleter {}.complete(
                &CompletionContext(&context),
                "$nu.env.CO",
                &CaseInsensitiveMatcher
            ),
            vec![S::new("$nu.env.COMPLETER", "$nu.env.COMPLETER"),]
        );

        assert_eq!(
            VariableCompleter {}.complete(
                &CompletionContext(&context),
                "$nu.en",
                &CaseInsensitiveMatcher
            ),
            vec![S::new("$nu.env", "$nu.env"),]
        );
    }
}
