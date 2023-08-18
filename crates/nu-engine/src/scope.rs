use nu_protocol::{
    engine::{Command, EngineState, Stack, Visibility},
    ModuleId, Signature, Span, SyntaxShape, Type, Value,
};
use std::cmp::Ordering;
use std::collections::HashMap;

pub struct ScopeData<'e, 's> {
    engine_state: &'e EngineState,
    stack: &'s Stack,
    vars_map: HashMap<&'e Vec<u8>, &'e usize>,
    decls_map: HashMap<&'e Vec<u8>, &'e usize>,
    modules_map: HashMap<&'e Vec<u8>, &'e usize>,
    visibility: Visibility,
}

impl<'e, 's> ScopeData<'e, 's> {
    pub fn new(engine_state: &'e EngineState, stack: &'s Stack) -> Self {
        Self {
            engine_state,
            stack,
            vars_map: HashMap::new(),
            decls_map: HashMap::new(),
            modules_map: HashMap::new(),
            visibility: Visibility::new(),
        }
    }

    pub fn populate_vars(&mut self) {
        for overlay_frame in self.engine_state.active_overlays(&[]) {
            self.vars_map.extend(&overlay_frame.vars);
        }
    }

    // decls include all commands, i.e., normal commands, aliases, and externals
    pub fn populate_decls(&mut self) {
        for overlay_frame in self.engine_state.active_overlays(&[]) {
            self.decls_map.extend(&overlay_frame.decls);
            self.visibility.merge_with(overlay_frame.visibility.clone());
        }
    }

    pub fn populate_modules(&mut self) {
        for overlay_frame in self.engine_state.active_overlays(&[]) {
            self.modules_map.extend(&overlay_frame.modules);
        }
    }

    pub fn collect_vars(&self, span: Span) -> Vec<Value> {
        let mut vars = vec![];

        for (var_name, var_id) in &self.vars_map {
            let var_name = Value::string(String::from_utf8_lossy(var_name).to_string(), span);

            let var_type = Value::string(self.engine_state.get_var(**var_id).ty.to_string(), span);

            let var_value = if let Ok(val) = self.stack.get_var(**var_id, span) {
                val
            } else {
                Value::nothing(span)
            };

            let var_id_val = Value::int(**var_id as i64, span);

            vars.push(Value::Record {
                cols: vec![
                    "name".to_string(),
                    "type".to_string(),
                    "value".to_string(),
                    "var_id".to_string(),
                ],
                vals: vec![var_name, var_type, var_value, var_id_val],
                span,
            })
        }

        sort_rows(&mut vars);
        vars
    }

    pub fn collect_commands(&self, span: Span) -> Vec<Value> {
        let mut commands = vec![];

        for (command_name, decl_id) in &self.decls_map {
            if self.visibility.is_decl_id_visible(decl_id)
                && !self.engine_state.get_decl(**decl_id).is_alias()
            {
                let mut cols = vec![];
                let mut vals = vec![];

                cols.push("name".into());
                vals.push(Value::String {
                    val: String::from_utf8_lossy(command_name).to_string(),
                    span,
                });

                let decl = self.engine_state.get_decl(**decl_id);
                let signature = decl.signature();

                cols.push("category".to_string());
                vals.push(Value::String {
                    val: signature.category.to_string(),
                    span,
                });

                cols.push("signatures".to_string());
                vals.push(self.collect_signatures(&signature, span));

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
                            cols: vec!["description".into(), "example".into(), "result".into()],
                            vals: vec![
                                Value::String {
                                    val: x.description.to_string(),
                                    span,
                                },
                                Value::String {
                                    val: x.example.to_string(),
                                    span,
                                },
                                if let Some(result) = x.result {
                                    result
                                } else {
                                    Value::Nothing { span }
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
                vals.push(Value::String {
                    val: search_terms.join(", "),
                    span,
                });

                cols.push("decl_id".into());
                vals.push(Value::int(**decl_id as i64, span));

                commands.push(Value::Record { cols, vals, span })
            }
        }

        sort_rows(&mut commands);

        commands
    }

    fn collect_signatures(&self, signature: &Signature, span: Span) -> Value {
        let mut sigs = signature
            .input_output_types
            .iter()
            .map(|(input_type, output_type)| {
                (
                    input_type.to_shape().to_string(),
                    Value::List {
                        vals: self.collect_signature_entries(
                            input_type,
                            output_type,
                            signature,
                            span,
                        ),
                        span,
                    },
                )
            })
            .collect::<Vec<(String, Value)>>();

        // Until we allow custom commands to have input and output types, let's just
        // make them Type::Any Type::Any so they can show up in our `scope commands`
        // a little bit better. If sigs is empty, we're pretty sure that we're dealing
        // with a custom command.
        if sigs.is_empty() {
            let any_type = &Type::Any;
            sigs.push((
                any_type.to_shape().to_string(),
                Value::List {
                    vals: self.collect_signature_entries(any_type, any_type, signature, span),
                    span,
                },
            ));
        }
        sigs.sort_unstable_by(|(k1, _), (k2, _)| k1.cmp(k2));
        // For most commands, input types are not repeated in
        // `input_output_types`, i.e. each input type has only one associated
        // output type. Furthermore, we want this to always be true. However,
        // there are currently some exceptions, such as `hash sha256` which
        // takes in string but may output string or binary depending on the
        // presence of the --binary flag. In such cases, the "special case"
        // signature usually comes later in the input_output_types, so this will
        // remove them from the record.
        sigs.dedup_by(|(k1, _), (k2, _)| k1 == k2);
        let (cols, vals) = sigs.into_iter().unzip();
        Value::Record { cols, vals, span }
    }

    fn collect_signature_entries(
        &self,
        input_type: &Type,
        output_type: &Type,
        signature: &Signature,
        span: Span,
    ) -> Vec<Value> {
        let mut sig_records = vec![];

        let sig_cols = vec![
            "parameter_name".to_string(),
            "parameter_type".to_string(),
            "syntax_shape".to_string(),
            "is_optional".to_string(),
            "short_flag".to_string(),
            "description".to_string(),
            "custom_completion".to_string(),
            "parameter_default".to_string(),
        ];

        // input
        sig_records.push(Value::Record {
            cols: sig_cols.clone(),
            vals: vec![
                Value::nothing(span),
                Value::string("input", span),
                Value::string(input_type.to_shape().to_string(), span),
                Value::bool(false, span),
                Value::nothing(span),
                Value::nothing(span),
                Value::nothing(span),
                Value::nothing(span),
            ],
            span,
        });

        // required_positional
        for req in &signature.required_positional {
            let sig_vals = vec![
                Value::string(&req.name, span),
                Value::string("positional", span),
                Value::string(req.shape.to_string(), span),
                Value::bool(false, span),
                Value::nothing(span),
                Value::string(&req.desc, span),
                Value::string(
                    extract_custom_completion_from_arg(self.engine_state, &req.shape),
                    span,
                ),
                Value::nothing(span),
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
                Value::string(&opt.name, span),
                Value::string("positional", span),
                Value::string(opt.shape.to_string(), span),
                Value::bool(true, span),
                Value::nothing(span),
                Value::string(&opt.desc, span),
                Value::string(
                    extract_custom_completion_from_arg(self.engine_state, &opt.shape),
                    span,
                ),
                if let Some(val) = &opt.default_value {
                    val.clone()
                } else {
                    Value::nothing(span)
                },
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
                Value::string(if rest.name == "rest" { "" } else { &rest.name }, span),
                Value::string("rest", span),
                Value::string(rest.shape.to_string(), span),
                Value::bool(true, span),
                Value::nothing(span),
                Value::string(&rest.desc, span),
                Value::string(
                    extract_custom_completion_from_arg(self.engine_state, &rest.shape),
                    span,
                ),
                Value::nothing(span), // rest_positional does have default, but parser prohibits specifying it?!
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
                Value::string(&named.long, span),
                flag_type,
                shape,
                Value::bool(!named.required, span),
                short_flag,
                Value::string(&named.desc, span),
                Value::string(custom_completion_command_name, span),
                if let Some(val) = &named.default_value {
                    val.clone()
                } else {
                    Value::nothing(span)
                },
            ];

            sig_records.push(Value::Record {
                cols: sig_cols.clone(),
                vals: sig_vals,
                span,
            });
        }

        // output
        sig_records.push(Value::Record {
            cols: sig_cols,
            vals: vec![
                Value::nothing(span),
                Value::string("output", span),
                Value::string(output_type.to_shape().to_string(), span),
                Value::bool(false, span),
                Value::nothing(span),
                Value::nothing(span),
                Value::nothing(span),
                Value::nothing(span),
            ],
            span,
        });

        sig_records
    }

    pub fn collect_externs(&self, span: Span) -> Vec<Value> {
        let mut externals = vec![];

        for (command_name, decl_id) in &self.decls_map {
            let decl = self.engine_state.get_decl(**decl_id);

            if decl.is_known_external() {
                let mut cols = vec![];
                let mut vals = vec![];

                cols.push("name".into());
                vals.push(Value::String {
                    val: String::from_utf8_lossy(command_name).to_string(),
                    span,
                });

                cols.push("usage".to_string());
                vals.push(Value::String {
                    val: decl.usage().into(),
                    span,
                });

                cols.push("decl_id".into());
                vals.push(Value::int(**decl_id as i64, span));

                externals.push(Value::Record { cols, vals, span })
            }
        }

        sort_rows(&mut externals);
        externals
    }

    pub fn collect_aliases(&self, span: Span) -> Vec<Value> {
        let mut aliases = vec![];

        for (decl_name, decl_id) in self.engine_state.get_decls_sorted(false) {
            if self.visibility.is_decl_id_visible(&decl_id) {
                let decl = self.engine_state.get_decl(decl_id);
                if let Some(alias) = decl.as_alias() {
                    aliases.push(Value::Record {
                        cols: vec![
                            "name".into(),
                            "expansion".into(),
                            "usage".into(),
                            "decl_id".into(),
                        ],
                        vals: vec![
                            Value::String {
                                val: String::from_utf8_lossy(&decl_name).to_string(),
                                span,
                            },
                            Value::String {
                                val: String::from_utf8_lossy(
                                    self.engine_state.get_span_contents(alias.wrapped_call.span),
                                )
                                .to_string(),
                                span,
                            },
                            Value::String {
                                val: alias.usage().to_string(),
                                span,
                            },
                            Value::Int {
                                val: decl_id as i64,
                                span,
                            },
                        ],
                        span,
                    });
                }
            }
        }

        sort_rows(&mut aliases);
        // aliases.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        aliases
    }

    fn collect_module(&self, module_name: &[u8], module_id: &ModuleId, span: Span) -> Value {
        let module = self.engine_state.get_module(*module_id);

        let all_decls = module.decls();

        let mut export_commands: Vec<Value> = all_decls
            .iter()
            .filter_map(|(name_bytes, decl_id)| {
                let decl = self.engine_state.get_decl(*decl_id);

                if !decl.is_alias() && !decl.is_known_external() {
                    Some(Value::record(
                        vec!["name".into(), "decl_id".into()],
                        vec![
                            Value::string(String::from_utf8_lossy(name_bytes), span),
                            Value::int(*decl_id as i64, span),
                        ],
                        span,
                    ))
                } else {
                    None
                }
            })
            .collect();

        let mut export_aliases: Vec<Value> = all_decls
            .iter()
            .filter_map(|(name_bytes, decl_id)| {
                let decl = self.engine_state.get_decl(*decl_id);

                if decl.is_alias() {
                    Some(Value::record(
                        vec!["name".into(), "decl_id".into()],
                        vec![
                            Value::string(String::from_utf8_lossy(name_bytes), span),
                            Value::int(*decl_id as i64, span),
                        ],
                        span,
                    ))
                } else {
                    None
                }
            })
            .collect();

        let mut export_externs: Vec<Value> = all_decls
            .iter()
            .filter_map(|(name_bytes, decl_id)| {
                let decl = self.engine_state.get_decl(*decl_id);

                if decl.is_known_external() {
                    Some(Value::record(
                        vec!["name".into(), "decl_id".into()],
                        vec![
                            Value::string(String::from_utf8_lossy(name_bytes), span),
                            Value::int(*decl_id as i64, span),
                        ],
                        span,
                    ))
                } else {
                    None
                }
            })
            .collect();

        let mut export_submodules: Vec<Value> = module
            .submodules()
            .iter()
            .map(|(name_bytes, submodule_id)| self.collect_module(name_bytes, submodule_id, span))
            .collect();

        let mut export_consts: Vec<Value> = module
            .vars()
            .iter()
            .map(|(name_bytes, var_id)| {
                Value::record(
                    vec!["name".into(), "type".into(), "var_id".into()],
                    vec![
                        Value::string(String::from_utf8_lossy(name_bytes), span),
                        Value::string(self.engine_state.get_var(*var_id).ty.to_string(), span),
                        Value::int(*var_id as i64, span),
                    ],
                    span,
                )
            })
            .collect();

        sort_rows(&mut export_commands);
        sort_rows(&mut export_aliases);
        sort_rows(&mut export_externs);
        sort_rows(&mut export_submodules);
        sort_rows(&mut export_consts);

        let export_env_block = module.env_block.map_or_else(
            || Value::nothing(span),
            |block_id| Value::Block {
                val: block_id,
                span,
            },
        );

        let module_usage = self
            .engine_state
            .build_module_usage(*module_id)
            .map(|(usage, _)| usage)
            .unwrap_or_default();

        Value::Record {
            cols: vec![
                "name".into(),
                "commands".into(),
                "aliases".into(),
                "externs".into(),
                "submodules".into(),
                "constants".into(),
                "env_block".into(),
                "usage".into(),
                "module_id".into(),
            ],
            vals: vec![
                Value::string(String::from_utf8_lossy(module_name), span),
                Value::List {
                    vals: export_commands,
                    span,
                },
                Value::List {
                    vals: export_aliases,
                    span,
                },
                Value::List {
                    vals: export_externs,
                    span,
                },
                Value::List {
                    vals: export_submodules,
                    span,
                },
                Value::List {
                    vals: export_consts,
                    span,
                },
                export_env_block,
                Value::string(module_usage, span),
                Value::int(*module_id as i64, span),
            ],
            span,
        }
    }

    pub fn collect_modules(&self, span: Span) -> Vec<Value> {
        let mut modules = vec![];

        for (module_name, module_id) in &self.modules_map {
            modules.push(self.collect_module(module_name, module_id, span));
        }

        modules.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        modules
    }

    pub fn collect_engine_state(&self, span: Span) -> Value {
        let engine_state_cols = vec![
            "source_bytes".to_string(),
            "num_vars".to_string(),
            "num_decls".to_string(),
            "num_blocks".to_string(),
            "num_modules".to_string(),
            "num_env_vars".to_string(),
        ];

        let engine_state_vals = vec![
            Value::int(self.engine_state.next_span_start() as i64, span),
            Value::int(self.engine_state.num_vars() as i64, span),
            Value::int(self.engine_state.num_decls() as i64, span),
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

fn sort_rows(decls: &mut [Value]) {
    decls.sort_by(|a, b| match (a, b) {
        (Value::Record { vals: rec_a, .. }, Value::Record { vals: rec_b, .. }) => {
            // Comparing the first value from the record
            // It is expected that the first value is the name of the entry (command, module, alias, etc.)
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
}
