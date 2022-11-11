use nu_protocol::{
    engine::{EngineState, Stack, Visibility},
    ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use std::cmp::Ordering;
use std::collections::HashMap;

pub fn create_scope(
    engine_state: &EngineState,
    stack: &Stack,
    span: Span,
) -> Result<Value, ShellError> {
    let mut scope_data = ScopeData::new(engine_state, stack);

    scope_data.populate_from_overlays();

    let mut cols = vec![];
    let mut vals = vec![];

    cols.push("vars".to_string());
    vals.push(Value::List {
        vals: scope_data.collect_vars(span),
        span,
    });

    cols.push("commands".to_string());
    vals.push(Value::List {
        vals: scope_data.collect_commands(span),
        span,
    });

    cols.push("aliases".to_string());
    vals.push(Value::List {
        vals: scope_data
            .collect_aliases(span)
            .into_iter()
            .map(|(alias, value)| Value::Record {
                cols: vec!["alias".into(), "expansion".into()],
                vals: vec![alias, value],
                span,
            })
            .collect(),
        span,
    });

    cols.push("modules".to_string());
    vals.push(Value::List {
        vals: scope_data.collect_modules(span),
        span,
    });

    cols.push("engine_state".to_string());
    vals.push(scope_data.collect_engine_state(span));

    Ok(Value::Record { cols, vals, span })
}

struct ScopeData<'e, 's> {
    engine_state: &'e EngineState,
    stack: &'s Stack,
    vars_map: HashMap<&'e Vec<u8>, &'e usize>,
    commands_map: HashMap<&'e (Vec<u8>, Type), &'e usize>,
    aliases_map: HashMap<&'e Vec<u8>, &'e usize>,
    modules_map: HashMap<&'e Vec<u8>, &'e usize>,
    visibility: Visibility,
}

impl<'e, 's> ScopeData<'e, 's> {
    pub fn new(engine_state: &'e EngineState, stack: &'s Stack) -> Self {
        Self {
            engine_state,
            stack,
            vars_map: HashMap::new(),
            commands_map: HashMap::new(),
            aliases_map: HashMap::new(),
            modules_map: HashMap::new(),
            visibility: Visibility::new(),
        }
    }

    pub fn populate_from_overlays(&mut self) {
        for overlay_frame in self.engine_state.active_overlays(&[]) {
            self.vars_map.extend(&overlay_frame.vars);
            self.commands_map.extend(&overlay_frame.decls);
            self.aliases_map.extend(&overlay_frame.aliases);
            self.modules_map.extend(&overlay_frame.modules);
            self.visibility.merge_with(overlay_frame.visibility.clone());
        }
    }

    pub fn collect_vars(&mut self, span: Span) -> Vec<Value> {
        let mut vars = vec![];
        for var in &self.vars_map {
            let var_name = Value::string(String::from_utf8_lossy(var.0).to_string(), span);

            let var_type = Value::string(self.engine_state.get_var(**var.1).ty.to_string(), span);

            let var_value = if let Ok(val) = self.stack.get_var(**var.1, span) {
                val
            } else {
                Value::nothing(span)
            };

            vars.push(Value::Record {
                cols: vec!["name".to_string(), "type".to_string(), "value".to_string()],
                vals: vec![var_name, var_type, var_value],
                span,
            })
        }
        vars
    }

    pub fn collect_commands(&mut self, span: Span) -> Vec<Value> {
        let mut commands = vec![];
        for ((command_name, _), decl_id) in &self.commands_map {
            if self.visibility.is_decl_id_visible(decl_id) {
                let mut cols = vec![];
                let mut vals = vec![];

                let mut module_commands = vec![];
                for module in &self.modules_map {
                    let module_name = String::from_utf8_lossy(module.0).to_string();
                    let module_id = self.engine_state.find_module(module.0, &[]);
                    if let Some(module_id) = module_id {
                        let module = self.engine_state.get_module(module_id);
                        if module.has_decl(command_name) {
                            module_commands.push(module_name);
                        }
                    }
                }

                cols.push("name".into());
                vals.push(Value::String {
                    val: String::from_utf8_lossy(command_name).to_string(),
                    span,
                });

                cols.push("module_name".into());
                vals.push(Value::string(module_commands.join(", "), span));

                let decl = self.engine_state.get_decl(**decl_id);
                let signature = decl.signature();

                cols.push("category".to_string());
                vals.push(Value::String {
                    val: signature.category.to_string(),
                    span,
                });

                cols.push("signature".to_string());
                vals.push(Value::List {
                    vals: self.collect_signature_entries(&signature, span),
                    span,
                });

                cols.push("usage".to_string());
                vals.push(Value::String {
                    val: decl.usage().into(),
                    span,
                });

                cols.push("examples".to_string());
                vals.push(Value::List {
                    vals: decl
                        .examples()
                        .into_iter()
                        .map(|x| Value::Record {
                            cols: vec!["description".into(), "example".into()],
                            vals: vec![
                                Value::String {
                                    val: x.description.to_string(),
                                    span,
                                },
                                Value::String {
                                    val: x.example.to_string(),
                                    span,
                                },
                            ],
                            span,
                        })
                        .collect(),
                    span,
                });

                cols.push("is_builtin".to_string());
                // we can only be a is_builtin or is_custom, not both
                vals.push(Value::Bool {
                    val: !decl.is_custom_command(),
                    span,
                });

                cols.push("is_sub".to_string());
                vals.push(Value::Bool {
                    val: decl.is_sub(),
                    span,
                });

                cols.push("is_plugin".to_string());
                vals.push(Value::Bool {
                    val: decl.is_plugin().is_some(),
                    span,
                });

                cols.push("is_custom".to_string());
                vals.push(Value::Bool {
                    val: decl.is_custom_command(),
                    span,
                });

                cols.push("is_keyword".into());
                vals.push(Value::Bool {
                    val: decl.is_parser_keyword(),
                    span,
                });

                cols.push("is_extern".to_string());
                vals.push(Value::Bool {
                    val: decl.is_known_external(),
                    span,
                });

                cols.push("creates_scope".to_string());
                vals.push(Value::Bool {
                    val: signature.creates_scope,
                    span,
                });

                cols.push("extra_usage".to_string());
                vals.push(Value::String {
                    val: decl.extra_usage().into(),
                    span,
                });

                let search_terms = decl.search_terms();
                cols.push("search_terms".to_string());
                vals.push(if search_terms.is_empty() {
                    Value::nothing(span)
                } else {
                    Value::String {
                        val: search_terms.join(", "),
                        span,
                    }
                });

                commands.push(Value::Record { cols, vals, span })
            }
        }

        commands.sort_by(|a, b| match (a, b) {
            (Value::Record { vals: rec_a, .. }, Value::Record { vals: rec_b, .. }) => {
                // Comparing the first value from the record
                // It is expected that the first value is the name of the column
                // The names of the commands should be a value string
                match (rec_a.get(0), rec_b.get(0)) {
                    (Some(val_a), Some(val_b)) => match (val_a, val_b) {
                        (Value::String { val: str_a, .. }, Value::String { val: str_b, .. }) => {
                            str_a.cmp(str_b)
                        }
                        _ => Ordering::Equal,
                    },
                    _ => Ordering::Equal,
                }
            }
            _ => Ordering::Equal,
        });
        commands
    }

    fn collect_signature_entries(&self, signature: &Signature, span: Span) -> Vec<Value> {
        let mut sig_records = vec![];

        let sig_cols = vec![
            "command".to_string(),
            "parameter_name".to_string(),
            "parameter_type".to_string(),
            "syntax_shape".to_string(),
            "is_optional".to_string(),
            "short_flag".to_string(),
            "description".to_string(),
            "custom_completion".to_string(),
        ];

        // required_positional
        for req in &signature.required_positional {
            let sig_vals = vec![
                Value::string(&signature.name, span),
                Value::string(&req.name, span),
                Value::string("positional", span),
                Value::string(req.shape.to_string(), span),
                Value::boolean(false, span),
                Value::nothing(span),
                Value::string(&req.desc, span),
                Value::string(
                    extract_custom_completion_from_arg(self.engine_state, &req.shape),
                    span,
                ),
            ];

            sig_records.push(Value::Record {
                cols: sig_cols.clone(),
                vals: sig_vals,
                span,
            });
        }

        // optional_positional
        for opt in &signature.optional_positional {
            let sig_vals = vec![
                Value::string(&signature.name, span),
                Value::string(&opt.name, span),
                Value::string("positional", span),
                Value::string(opt.shape.to_string(), span),
                Value::boolean(true, span),
                Value::nothing(span),
                Value::string(&opt.desc, span),
                Value::string(
                    extract_custom_completion_from_arg(self.engine_state, &opt.shape),
                    span,
                ),
            ];

            sig_records.push(Value::Record {
                cols: sig_cols.clone(),
                vals: sig_vals,
                span,
            });
        }

        // rest_positional
        if let Some(rest) = &signature.rest_positional {
            let sig_vals = vec![
                Value::string(&signature.name, span),
                Value::string(&rest.name, span),
                Value::string("rest", span),
                Value::string(rest.shape.to_string(), span),
                Value::boolean(true, span),
                Value::nothing(span),
                Value::string(&rest.desc, span),
                Value::string(
                    extract_custom_completion_from_arg(self.engine_state, &rest.shape),
                    span,
                ),
            ];

            sig_records.push(Value::Record {
                cols: sig_cols.clone(),
                vals: sig_vals,
                span,
            });
        }

        // named flags
        for named in &signature.named {
            let flag_type;

            // Skip the help flag
            if named.long == "help" {
                continue;
            }

            let mut custom_completion_command_name: String = "".to_string();
            let shape = if let Some(arg) = &named.arg {
                flag_type = Value::string("named", span);
                custom_completion_command_name =
                    extract_custom_completion_from_arg(self.engine_state, arg);
                Value::string(arg.to_string(), span)
            } else {
                flag_type = Value::string("switch", span);
                Value::nothing(span)
            };

            let short_flag = if let Some(c) = named.short {
                Value::string(c, span)
            } else {
                Value::nothing(span)
            };

            let sig_vals = vec![
                Value::string(&signature.name, span),
                Value::string(&named.long, span),
                flag_type,
                shape,
                Value::boolean(!named.required, span),
                short_flag,
                Value::string(&named.desc, span),
                Value::string(custom_completion_command_name, span),
            ];

            sig_records.push(Value::Record {
                cols: sig_cols.clone(),
                vals: sig_vals,
                span,
            });
        }
        sig_records
    }

    pub fn collect_aliases(&mut self, span: Span) -> Vec<(Value, Value)> {
        let mut aliases = vec![];
        for (alias_name, alias_id) in &self.aliases_map {
            if self.visibility.is_alias_id_visible(alias_id) {
                let alias = self.engine_state.get_alias(**alias_id);
                let mut alias_text = String::new();
                for span in alias {
                    let contents = self.engine_state.get_span_contents(span);
                    if !alias_text.is_empty() {
                        alias_text.push(' ');
                    }
                    alias_text.push_str(&String::from_utf8_lossy(contents));
                }
                aliases.push((
                    Value::String {
                        val: String::from_utf8_lossy(alias_name).to_string(),
                        span,
                    },
                    Value::string(alias_text, span),
                ));
            }
        }

        aliases.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        aliases
    }

    pub fn collect_modules(&mut self, span: Span) -> Vec<Value> {
        let mut modules = vec![];

        for module in &self.modules_map {
            modules.push(Value::String {
                val: String::from_utf8_lossy(module.0).to_string(),
                span,
            });
        }
        modules.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        modules
    }

    pub fn collect_engine_state(&mut self, span: Span) -> Value {
        let engine_state_cols = vec![
            "source_bytes".to_string(),
            "num_vars".to_string(),
            "num_commands".to_string(),
            "num_aliases".to_string(),
            "num_blocks".to_string(),
            "num_modules".to_string(),
            "num_env_vars".to_string(),
        ];

        let engine_state_vals = vec![
            Value::int(self.engine_state.next_span_start() as i64, span),
            Value::int(self.engine_state.num_vars() as i64, span),
            Value::int(self.engine_state.num_decls() as i64, span),
            Value::int(self.engine_state.num_aliases() as i64, span),
            Value::int(self.engine_state.num_blocks() as i64, span),
            Value::int(self.engine_state.num_modules() as i64, span),
            Value::int(
                self.engine_state
                    .env_vars
                    .values()
                    .map(|overlay| overlay.len() as i64)
                    .sum(),
                span,
            ),
        ];
        Value::Record {
            cols: engine_state_cols,
            vals: engine_state_vals,
            span,
        }
    }
}

fn extract_custom_completion_from_arg(engine_state: &EngineState, shape: &SyntaxShape) -> String {
    return match shape {
        SyntaxShape::Custom(_, custom_completion_decl_id) => {
            let custom_completion_command = engine_state.get_decl(*custom_completion_decl_id);
            let custom_completion_command_name: &str = custom_completion_command.name();
            custom_completion_command_name.to_string()
        }
        _ => "".to_string(),
    };
}
