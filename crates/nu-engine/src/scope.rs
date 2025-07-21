use nu_protocol::{
    DeclId, ModuleId, Signature, Span, Type, Value, VarId,
    ast::Expr,
    engine::{Command, EngineState, Stack, Visibility},
    record,
};
use std::{cmp::Ordering, collections::HashMap};

pub struct ScopeData<'e, 's> {
    engine_state: &'e EngineState,
    stack: &'s Stack,
    vars_map: HashMap<&'e Vec<u8>, &'e VarId>,
    decls_map: HashMap<&'e Vec<u8>, &'e DeclId>,
    modules_map: HashMap<&'e Vec<u8>, &'e ModuleId>,
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

            let var = self.engine_state.get_var(**var_id);
            let var_type = Value::string(var.ty.to_string(), span);
            let is_const = Value::bool(var.const_val.is_some(), span);

            let var_value = self
                .stack
                .get_var(**var_id, span)
                .ok()
                .or(var.const_val.clone())
                .unwrap_or(Value::nothing(span));

            let var_id_val = Value::int(var_id.get() as i64, span);

            vars.push(Value::record(
                record! {
                    "name" => var_name,
                    "type" => var_type,
                    "value" => var_value,
                    "is_const" => is_const,
                    "var_id" => var_id_val,
                },
                span,
            ));
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
                let decl = self.engine_state.get_decl(**decl_id);
                let signature = decl.signature();

                let examples = decl
                    .examples()
                    .into_iter()
                    .map(|x| {
                        Value::record(
                            record! {
                                "description" => Value::string(x.description, span),
                                "example" => Value::string(x.example, span),
                                "result" => x.result.unwrap_or(Value::nothing(span)).with_span(span),
                            },
                            span,
                        )
                    })
                    .collect();

                let attributes = decl
                    .attributes()
                    .into_iter()
                    .map(|(name, value)| {
                        Value::record(
                            record! {
                                "name" => Value::string(name, span),
                                "value" => value,
                            },
                            span,
                        )
                    })
                    .collect();

                let record = record! {
                    "name" => Value::string(String::from_utf8_lossy(command_name), span),
                    "category" => Value::string(signature.category.to_string(), span),
                    "signatures" => self.collect_signatures(&signature, span),
                    "description" => Value::string(decl.description(), span),
                    "examples" => Value::list(examples, span),
                    "attributes" => Value::list(attributes, span),
                    "type" => Value::string(decl.command_type().to_string(), span),
                    "is_sub" => Value::bool(decl.is_sub(), span),
                    "is_const" => Value::bool(decl.is_const(), span),
                    "creates_scope" => Value::bool(signature.creates_scope, span),
                    "extra_description" => Value::string(decl.extra_description(), span),
                    "search_terms" => Value::string(decl.search_terms().join(", "), span),
                    "decl_id" => Value::int(decl_id.get() as i64, span),
                };

                commands.push(Value::record(record, span))
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
                    Value::list(
                        self.collect_signature_entries(input_type, output_type, signature, span),
                        span,
                    ),
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
                Value::list(
                    self.collect_signature_entries(any_type, any_type, signature, span),
                    span,
                ),
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
        Value::record(sigs.into_iter().collect(), span)
    }

    fn collect_signature_entries(
        &self,
        input_type: &Type,
        output_type: &Type,
        signature: &Signature,
        span: Span,
    ) -> Vec<Value> {
        let mut sig_records = vec![];

        // input
        sig_records.push(Value::record(
            record! {
                "parameter_name" => Value::nothing(span),
                "parameter_type" => Value::string("input", span),
                "syntax_shape" => Value::string(input_type.to_shape().to_string(), span),
                "is_optional" => Value::bool(false, span),
                "short_flag" => Value::nothing(span),
                "description" => Value::nothing(span),
                "completion" => Value::nothing(span),
                "parameter_default" => Value::nothing(span),
            },
            span,
        ));

        // required_positional
        for req in &signature.required_positional {
            let completion = req
                .completion
                .as_ref()
                .map(|compl| compl.to_value(self.engine_state, span))
                .unwrap_or(Value::nothing(span));

            sig_records.push(Value::record(
                record! {
                    "parameter_name" => Value::string(&req.name, span),
                    "parameter_type" => Value::string("positional", span),
                    "syntax_shape" => Value::string(req.shape.to_string(), span),
                    "is_optional" => Value::bool(false, span),
                    "short_flag" => Value::nothing(span),
                    "description" => Value::string(&req.desc, span),
                    "completion" => completion,
                    "parameter_default" => Value::nothing(span),
                },
                span,
            ));
        }

        // optional_positional
        for opt in &signature.optional_positional {
            let completion = opt
                .completion
                .as_ref()
                .map(|compl| compl.to_value(self.engine_state, span))
                .unwrap_or(Value::nothing(span));

            let default = if let Some(val) = &opt.default_value {
                val.clone()
            } else {
                Value::nothing(span)
            };

            sig_records.push(Value::record(
                record! {
                    "parameter_name" => Value::string(&opt.name, span),
                    "parameter_type" => Value::string("positional", span),
                    "syntax_shape" => Value::string(opt.shape.to_string(), span),
                    "is_optional" => Value::bool(true, span),
                    "short_flag" => Value::nothing(span),
                    "description" => Value::string(&opt.desc, span),
                    "completion" => completion,
                    "parameter_default" => default,
                },
                span,
            ));
        }

        // rest_positional
        if let Some(rest) = &signature.rest_positional {
            let name = if rest.name == "rest" { "" } else { &rest.name };
            let completion = rest
                .completion
                .as_ref()
                .map(|compl| compl.to_value(self.engine_state, span))
                .unwrap_or(Value::nothing(span));

            sig_records.push(Value::record(
                record! {
                    "parameter_name" => Value::string(name, span),
                    "parameter_type" => Value::string("rest", span),
                    "syntax_shape" => Value::string(rest.shape.to_string(), span),
                    "is_optional" => Value::bool(true, span),
                    "short_flag" => Value::nothing(span),
                    "description" => Value::string(&rest.desc, span),
                    "completion" => completion,
                    // rest_positional does have default, but parser prohibits specifying it?!
                    "parameter_default" => Value::nothing(span),
                },
                span,
            ));
        }

        // named flags
        for named in &signature.named {
            let flag_type;

            // Skip the help flag
            if named.long == "help" {
                continue;
            }

            let completion = named
                .completion
                .as_ref()
                .map(|compl| compl.to_value(self.engine_state, span))
                .unwrap_or(Value::nothing(span));

            let shape = if let Some(arg) = &named.arg {
                flag_type = Value::string("named", span);
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

            let default = if let Some(val) = &named.default_value {
                val.clone()
            } else {
                Value::nothing(span)
            };

            sig_records.push(Value::record(
                record! {
                    "parameter_name" => Value::string(&named.long, span),
                    "parameter_type" => flag_type,
                    "syntax_shape" => shape,
                    "is_optional" => Value::bool(!named.required, span),
                    "short_flag" => short_flag,
                    "description" => Value::string(&named.desc, span),
                    "completion" => completion,
                    "parameter_default" => default,
                },
                span,
            ));
        }

        // output
        sig_records.push(Value::record(
            record! {
                "parameter_name" => Value::nothing(span),
                "parameter_type" => Value::string("output", span),
                "syntax_shape" => Value::string(output_type.to_shape().to_string(), span),
                "is_optional" => Value::bool(false, span),
                "short_flag" => Value::nothing(span),
                "description" => Value::nothing(span),
                "completion" => Value::nothing(span),
                "parameter_default" => Value::nothing(span),
            },
            span,
        ));

        sig_records
    }

    pub fn collect_externs(&self, span: Span) -> Vec<Value> {
        let mut externals = vec![];

        for (command_name, decl_id) in &self.decls_map {
            let decl = self.engine_state.get_decl(**decl_id);

            if decl.is_known_external() {
                let record = record! {
                    "name" => Value::string(String::from_utf8_lossy(command_name), span),
                    "description" => Value::string(decl.description(), span),
                    "decl_id" => Value::int(decl_id.get() as i64, span),
                };

                externals.push(Value::record(record, span))
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
                    let aliased_decl_id = if let Expr::Call(wrapped_call) = &alias.wrapped_call.expr
                    {
                        Value::int(wrapped_call.decl_id.get() as i64, span)
                    } else {
                        Value::nothing(span)
                    };

                    let expansion = String::from_utf8_lossy(
                        self.engine_state.get_span_contents(alias.wrapped_call.span),
                    );

                    aliases.push(Value::record(
                        record! {
                            "name" => Value::string(String::from_utf8_lossy(&decl_name), span),
                            "expansion" => Value::string(expansion, span),
                            "description" => Value::string(alias.description(), span),
                            "decl_id" => Value::int(decl_id.get() as i64, span),
                            "aliased_decl_id" => aliased_decl_id,
                        },
                        span,
                    ));
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
                        record! {
                            "name" => Value::string(String::from_utf8_lossy(name_bytes), span),
                            "decl_id" => Value::int(decl_id.get() as i64, span),
                        },
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
                        record! {
                            "name" => Value::string(String::from_utf8_lossy(name_bytes), span),
                            "decl_id" => Value::int(decl_id.get() as i64, span),
                        },
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
                        record! {
                            "name" => Value::string(String::from_utf8_lossy(name_bytes), span),
                            "decl_id" => Value::int(decl_id.get() as i64, span),
                        },
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
            .consts()
            .iter()
            .map(|(name_bytes, var_id)| {
                Value::record(
                    record! {
                        "name" => Value::string(String::from_utf8_lossy(name_bytes), span),
                        "type" => Value::string(self.engine_state.get_var(*var_id).ty.to_string(), span),
                        "var_id" => Value::int(var_id.get() as i64, span),
                    },
                    span,
                )
            })
            .collect();

        sort_rows(&mut export_commands);
        sort_rows(&mut export_aliases);
        sort_rows(&mut export_externs);
        sort_rows(&mut export_submodules);
        sort_rows(&mut export_consts);

        let (module_desc, module_extra_desc) = self
            .engine_state
            .build_module_desc(*module_id)
            .unwrap_or_default();

        Value::record(
            record! {
                "name" => Value::string(String::from_utf8_lossy(module_name), span),
                "commands" => Value::list(export_commands, span),
                "aliases" => Value::list(export_aliases, span),
                "externs" => Value::list(export_externs, span),
                "submodules" => Value::list(export_submodules, span),
                "constants" => Value::list(export_consts, span),
                "has_env_block" => Value::bool(module.env_block.is_some(), span),
                "description" => Value::string(module_desc, span),
                "extra_description" => Value::string(module_extra_desc, span),
                "module_id" => Value::int(module_id.get() as i64, span),
                "file" => Value::string(module.file.clone().map_or("unknown".to_string(), |(p, _)| p.path().to_string_lossy().to_string()), span),
            },
            span,
        )
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
        let num_env_vars = self
            .engine_state
            .env_vars
            .values()
            .map(|overlay| overlay.len() as i64)
            .sum();

        Value::record(
            record! {
                "source_bytes" => Value::int(self.engine_state.next_span_start() as i64, span),
                "num_vars" => Value::int(self.engine_state.num_vars() as i64, span),
                "num_decls" => Value::int(self.engine_state.num_decls() as i64, span),
                "num_blocks" => Value::int(self.engine_state.num_blocks() as i64, span),
                "num_modules" => Value::int(self.engine_state.num_modules() as i64, span),
                "num_env_vars" => Value::int(num_env_vars, span),
            },
            span,
        )
    }
}

fn sort_rows(decls: &mut [Value]) {
    decls.sort_by(|a, b| match (a, b) {
        (Value::Record { val: rec_a, .. }, Value::Record { val: rec_b, .. }) => {
            // Comparing the first value from the record
            // It is expected that the first value is the name of the entry (command, module, alias, etc.)
            match (rec_a.values().next(), rec_b.values().next()) {
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
