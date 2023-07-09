use nu_protocol::{
    engine::{Command, EngineState, Stack, Visibility},
    record, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;

pub fn create_scope(
    engine_state: &EngineState,
    stack: &Stack,
    span: Span,
) -> Result<Value, ShellError> {
    let mut scope_data = ScopeData::new(engine_state, stack);

    scope_data.populate_all();

    Ok(Value::record(
        record! {
            vars => Value::list(scope_data.collect_vars(span), span),
            commands => Value::list(scope_data.collect_commands(span), span),
            aliases => Value::list(scope_data.collect_aliases(span), span),
            modules => Value::list(scope_data.collect_modules(span), span),
            engine_state => scope_data.collect_engine_state(span),
        },
        span,
    ))
}

pub struct ScopeData<'e, 's> {
    engine_state: &'e EngineState,
    stack: &'s Stack,
    vars_map: HashMap<&'e Vec<u8>, &'e usize>,
    decls_map: HashMap<&'e (Vec<u8>, Type), &'e usize>,
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

    pub fn populate_all(&mut self) {
        for overlay_frame in self.engine_state.active_overlays(&[]) {
            self.vars_map.extend(&overlay_frame.vars);
            self.decls_map.extend(&overlay_frame.decls);
            self.modules_map.extend(&overlay_frame.modules);
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
        for var in &self.vars_map {
            let var_name = Value::string(String::from_utf8_lossy(var.0), span);

            let var_type = Value::string(self.engine_state.get_var(**var.1).ty.to_string(), span);

            let var_value = self
                .stack
                .get_var(**var.1, span)
                .unwrap_or(Value::nothing(span));

            vars.push(Value::record(
                record! {
                    name => var_name,
                    type => var_type,
                    value => var_value,
                },
                span,
            ))
        }
        vars
    }

    pub fn collect_commands(&self, span: Span) -> Vec<Value> {
        let mut commands = self
            .decls_map
            .iter()
            .filter_map(|((command_name, _), decl_id)| {
                if self.visibility.is_decl_id_visible(decl_id)
                    && !self.engine_state.get_decl(**decl_id).is_alias()
                {
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

                    let decl = self.engine_state.get_decl(**decl_id);
                    let signature = decl.signature();

                    let record = record! {
                        name => Value::string(String::from_utf8_lossy(command_name), span),
                        module_name => Value::string(module_commands.join(", "), span),
                        category => Value::string(signature.category.to_string(), span),
                        signatures => self.collect_signatures(&signature, span),
                        usage => Value::string(decl.usage(), span),
                        examples => Value::list(decl
                            .examples()
                            .into_iter()
                            .map(|x| {
                                Value::record(
                                    record! {
                                        description => Value::string(x.description, span),
                                        example => Value::string(x.example, span),
                                        result => x.result.unwrap_or(Value::nothing(span))
                                    },
                                    span,
                                )
                            })
                            .collect(), span),
                        is_builtin => Value::bool(!decl.is_custom_command(), span),
                        is_sub => Value::bool(!decl.is_sub(), span),
                        is_plugin => Value::bool(decl.is_plugin().is_some(), span),
                        is_custom => Value::bool(decl.is_custom_command(), span),
                        is_keyword => Value::bool(decl.is_parser_keyword(), span),
                        is_extern => Value::bool(decl.is_known_external(), span),
                        creates_scope => Value::bool(signature.creates_scope, span),
                        extra_usage => Value::string(decl.extra_usage(), span),
                        search_terms => Value::string(decl.search_terms().join(", "), span),
                    };

                    Some(Value::record(record, span))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        commands.sort_by(|a, b| match (a, b) {
            (Value::Record { val: rec_a, .. }, Value::Record { val: rec_b, .. }) => {
                // Comparing the first value from the record
                // It is expected that the first value is the name of the column
                // The names of the commands should be a value string
                match (rec_a.vals.get(0), rec_b.vals.get(0)) {
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
        Value::record_from_iter(sigs, span)
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
        sig_records.push(Value::record_from_parts(
            sig_cols.clone(),
            vec![
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
        ));

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

            sig_records.push(Value::record_from_parts(sig_cols.clone(), sig_vals, span));
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
                opt.default_value.clone().unwrap_or(Value::nothing(span)),
            ];

            sig_records.push(Value::record_from_parts(sig_cols.clone(), sig_vals, span));
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

            sig_records.push(Value::record_from_parts(sig_cols.clone(), sig_vals, span));
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

            sig_records.push(Value::record_from_parts(sig_cols.clone(), sig_vals, span));
        }

        // output
        sig_records.push(Value::record_from_parts(
            sig_cols,
            vec![
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
        ));

        sig_records
    }

    pub fn collect_externs(&self, span: Span) -> Vec<Value> {
        let mut externals = vec![];
        for ((command_name, _), decl_id) in &self.decls_map {
            let decl = self.engine_state.get_decl(**decl_id);

            if decl.is_known_external() {
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

                let record = record! {
                    name => Value::string(String::from_utf8_lossy(command_name), span),
                    module_name => Value::string(module_commands.join(", "), span),
                    usage => Value::string(decl.usage(), span)
                };

                externals.push(Value::record(record, span))
            }
        }

        externals
    }

    pub fn collect_aliases(&self, span: Span) -> Vec<Value> {
        let mut aliases = vec![];
        for (name_bytes, decl_id) in self.engine_state.get_decls_sorted(false) {
            if self.visibility.is_decl_id_visible(&decl_id) {
                let decl = self.engine_state.get_decl(decl_id);
                if let Some(alias) = decl.as_alias() {
                    let name = String::from_utf8_lossy(&name_bytes).to_string();
                    let sig = decl.signature().update_from_command(name, decl.borrow());
                    let expansion = String::from_utf8_lossy(
                        self.engine_state.get_span_contents(alias.wrapped_call.span),
                    );

                    aliases.push(Value::record(
                        record! {
                            name => Value::string(sig.name, span),
                            expansion => Value::string(expansion, span),
                            usage => Value::string(alias.signature().usage, span),
                        },
                        span,
                    ));
                }
            }
        }

        aliases.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        aliases
    }

    pub fn collect_modules(&self, span: Span) -> Vec<Value> {
        let mut modules = vec![];

        for (module_name, module_id) in &self.modules_map {
            let module = self.engine_state.get_module(**module_id);

            let export_commands: Vec<Value> = module
                .decls()
                .iter()
                .filter(|(_, id)| {
                    self.visibility.is_decl_id_visible(id)
                        && !self.engine_state.get_decl(*id).is_alias()
                })
                .map(|(bytes, _)| Value::string(String::from_utf8_lossy(bytes), span))
                .collect();

            let export_aliases: Vec<Value> = module
                .decls()
                .iter()
                .filter(|(_, id)| {
                    self.visibility.is_decl_id_visible(id)
                        && self.engine_state.get_decl(*id).is_alias()
                })
                .map(|(bytes, _)| Value::string(String::from_utf8_lossy(bytes), span))
                .collect();

            let export_env_block = module.env_block.map_or_else(
                || Value::nothing(span),
                |block_id| Value::Block {
                    val: block_id,
                    span,
                },
            );

            let module_usage = self
                .engine_state
                .build_module_usage(**module_id)
                .map(|(usage, _)| usage)
                .unwrap_or_default();

            modules.push(Value::record(
                record! {
                    name => Value::string(String::from_utf8_lossy(module_name), span),
                    commands => Value::list(export_commands, span),
                    aliases => Value::list(export_aliases, span),
                    env_block => export_env_block,
                    usage => Value::string(module_usage, span)
                },
                span,
            ));
        }
        modules.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        modules
    }

    pub fn collect_engine_state(&self, span: Span) -> Value {
        Value::record(
            record! {
                source_bytes => Value::int(self.engine_state.next_span_start() as i64, span),
                num_vars => Value::int(self.engine_state.num_vars() as i64, span),
                num_decls => Value::int(self.engine_state.num_decls() as i64, span),
                num_blocks => Value::int(self.engine_state.num_blocks() as i64, span),
                num_modules => Value::int(self.engine_state.num_modules() as i64, span),
                num_env_vars => Value::int(
                    self.engine_state
                        .env_vars
                        .values()
                        .map(|overlay| overlay.len() as i64)
                        .sum(),
                    span,
                )
            },
            span,
        )
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
