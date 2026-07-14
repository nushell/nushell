use crate::{
    lite_parser::LiteCommand,
    parse_def::has_flag_const,
    parse_helpers::{garbage, garbage_pipeline},
    parse_pipelines::{parse_redirection, redirecting_builtin_error},
    parser::{
        ArgumentParsingLevel, CallKind, ParsedInternalCall, compile_block, parse,
        parse_internal_call,
    },
};

use log::trace;
use nu_path::{absolute_with, is_windows_device_path};
#[cfg(feature = "plugin")]
use nu_protocol::ast::Call;
use nu_protocol::{
    BlockId, DeclId, ParseError, Span, Type, VarId,
    ast::{Block, Expr, Expression, Pipeline, PipelineElement},
    engine::StateWorkingSet,
    eval_const::eval_constant,
    parser_path::ParserPath,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub const LIB_DIRS_VAR: &str = "NU_LIB_DIRS";
#[cfg(feature = "plugin")]
pub const PLUGIN_DIRS_VAR: &str = "NU_PLUGIN_DIRS";

pub fn parse_source(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    trace!("parsing source");
    let spans = &lite_command.parts;
    let name = working_set.get_span_contents(spans[0]);

    if name == b"source" || name == b"source-env" {
        if let Some(redirection) = lite_command.redirection.as_ref() {
            let name = if name == b"source" {
                "source"
            } else {
                "source-env"
            };
            working_set.error(redirecting_builtin_error(name, redirection));
            return garbage_pipeline(working_set, spans);
        }

        let scoped = name == b"source-env";

        if let Some(decl_id) = working_set.find_decl(name) {
            #[allow(deprecated)]
            let cwd = working_set.get_cwd();

            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                ArgumentParsingLevel::Full,
                None,
            );

            if call_kind == CallKind::Help {
                return Pipeline::from_vec(vec![Expression::new(
                    working_set,
                    Expr::Call(call),
                    Span::concat(spans),
                    output,
                )]);
            }

            let first_expr = call.positional_iter().next();
            if let Some(expr) = first_expr {
                let val = match eval_constant(working_set, expr) {
                    Ok(val) => val,
                    Err(err) => {
                        working_set.error(err.wrap(working_set, Span::concat(&spans[1..])));
                        return Pipeline::from_vec(vec![Expression::new(
                            working_set,
                            Expr::Call(call),
                            Span::concat(&spans[1..]),
                            Type::Any,
                        )]);
                    }
                };

                if val.is_nothing() {
                    let mut call = call;
                    call.set_parser_info(
                        "noop".to_string(),
                        Expression::new_unknown(Expr::Nothing, Span::unknown(), Type::Nothing),
                    );
                    return Pipeline::from_vec(vec![Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(spans),
                        Type::Any,
                    )]);
                }

                let filename = match val.coerce_into_string() {
                    Ok(s) => s,
                    Err(err) => {
                        working_set.error(err.wrap(working_set, Span::concat(&spans[1..])));
                        return Pipeline::from_vec(vec![Expression::new(
                            working_set,
                            Expr::Call(call),
                            Span::concat(&spans[1..]),
                            Type::Any,
                        )]);
                    }
                };

                if let Some(path) = find_in_dirs(&filename, working_set, &cwd, Some(LIB_DIRS_VAR)) {
                    if let Some(contents) = path.read(working_set) {
                        if let Err(e) = working_set.files.push(path.clone().path_buf(), spans[1]) {
                            working_set.error(e);
                            return garbage_pipeline(working_set, spans);
                        }

                        let mut block = parse(
                            working_set,
                            Some(&path.path().to_string_lossy()),
                            &contents,
                            scoped,
                        );
                        if block.ir_block.is_none() {
                            let block_mut = Arc::make_mut(&mut block);
                            compile_block(working_set, block_mut);
                        }

                        working_set.files.pop();

                        let block_id = working_set.add_block(block);

                        let mut call_with_block = call;

                        call_with_block.set_parser_info(
                            "block_id".to_string(),
                            Expression::new(
                                working_set,
                                Expr::Int(block_id.get() as i64),
                                spans[1],
                                Type::Any,
                            ),
                        );

                        call_with_block.set_parser_info(
                            "block_id_name".to_string(),
                            Expression::new(
                                working_set,
                                Expr::Filepath(path.path_buf().display().to_string(), false),
                                spans[1],
                                Type::String,
                            ),
                        );

                        return Pipeline::from_vec(vec![Expression::new(
                            working_set,
                            Expr::Call(call_with_block),
                            Span::concat(spans),
                            Type::Any,
                        )]);
                    }
                } else {
                    working_set.error(ParseError::SourcedFileNotFound(filename, spans[1]));
                }
            }
            return Pipeline::from_vec(vec![Expression::new(
                working_set,
                Expr::Call(call),
                Span::concat(spans),
                Type::Any,
            )]);
        }
    }
    working_set.error(ParseError::UnknownState(
        "internal error: source statement unparsable".into(),
        Span::concat(spans),
    ));
    garbage_pipeline(working_set, spans)
}

pub fn parse_run(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    trace!("parsing run");
    let expr = parse_run_expr_internal(working_set, &lite_command.parts, lite_command);
    Pipeline::from_vec(vec![expr])
}

fn find_keyword_decl_by_name(working_set: &StateWorkingSet, name: &[u8]) -> Option<DeclId> {
    (0..working_set.num_decls())
        .map(DeclId::new)
        .find(|decl_id| {
            let decl = working_set.get_decl(*decl_id);
            decl.name().as_bytes() == name && decl.is_keyword()
        })
}

fn parse_run_expr_internal(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    lite_command: &LiteCommand,
) -> Expression {
    trace!("parsing run expression");
    let name = working_set.get_span_contents(spans.first().copied().unwrap_or(Span::unknown()));

    if name == b"run" {
        if let Some(redirection) = lite_command.redirection.as_ref() {
            working_set.error(redirecting_builtin_error("run", redirection));
            return garbage(working_set, Span::concat(spans));
        }

        if let Some(decl_id) =
            find_keyword_decl_by_name(working_set, name).or_else(|| working_set.find_decl(name))
        {
            #[allow(deprecated)]
            let cwd = working_set.get_cwd();

            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                ArgumentParsingLevel::Full,
                None,
            );

            if call_kind == CallKind::Help {
                return Expression::new(working_set, Expr::Call(call), Span::concat(spans), output);
            }

            let do_full_reparse = match has_flag_const(working_set, &call, "full-reparse") {
                Ok(value) => value,
                Err(()) => {
                    return Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(spans),
                        output,
                    );
                }
            };

            let first_expr = call.positional_iter().next();
            if let Some(expr) = first_expr {
                let val = match eval_constant(working_set, expr) {
                    Ok(val) => val,
                    Err(err) => {
                        working_set.error(err.wrap(working_set, Span::concat(&spans[1..])));
                        return Expression::new(
                            working_set,
                            Expr::Call(call),
                            Span::concat(&spans[1..]),
                            Type::Any,
                        );
                    }
                };

                if val.is_nothing() {
                    let mut call = call;
                    call.set_parser_info(
                        "noop".to_string(),
                        Expression::new_unknown(Expr::Nothing, Span::unknown(), Type::Nothing),
                    );
                    return Expression::new(
                        working_set,
                        Expr::Call(call),
                        Span::concat(spans),
                        Type::Any,
                    );
                }

                let filename = match val.coerce_into_string() {
                    Ok(s) => s,
                    Err(err) => {
                        working_set.error(err.wrap(working_set, Span::concat(&spans[1..])));
                        return Expression::new(
                            working_set,
                            Expr::Call(call),
                            Span::concat(&spans[1..]),
                            Type::Any,
                        );
                    }
                };

                if let Some(path) = find_in_dirs(&filename, working_set, &cwd, Some(LIB_DIRS_VAR)) {
                    if do_full_reparse {
                        let mut call_with_block = call;
                        call_with_block.set_parser_info(
                            "block_id_name".to_string(),
                            Expression::new(
                                working_set,
                                Expr::Filepath(path.path_buf().display().to_string(), false),
                                spans[1],
                                Type::String,
                            ),
                        );
                        call_with_block.set_parser_info(
                            "full_reparse".to_string(),
                            Expression::new(working_set, Expr::Bool(true), spans[1], Type::Bool),
                        );
                        return Expression::new(
                            working_set,
                            Expr::Call(call_with_block),
                            Span::concat(spans),
                            Type::Any,
                        );
                    }

                    if let Some(contents) = path.read(working_set) {
                        if let Err(e) = working_set.files.push(path.clone().path_buf(), spans[1]) {
                            working_set.error(e);
                            return garbage(working_set, Span::concat(spans));
                        }

                        let mut block = parse(
                            working_set,
                            Some(&path.path().to_string_lossy()),
                            &contents,
                            false,
                        );
                        if block.ir_block.is_none() {
                            let block_mut = Arc::make_mut(&mut block);
                            compile_block(working_set, block_mut);
                        }

                        working_set.files.pop();

                        let script_main_block_id =
                            find_main_block_id_in_script(working_set, &block);

                        let block_id = working_set.add_block(block);

                        let mut call_with_block = call;

                        call_with_block.set_parser_info(
                            "block_id".to_string(),
                            Expression::new(
                                working_set,
                                Expr::Int(block_id.get() as i64),
                                spans[1],
                                Type::Any,
                            ),
                        );

                        call_with_block.set_parser_info(
                            "block_id_name".to_string(),
                            Expression::new(
                                working_set,
                                Expr::Filepath(path.path_buf().display().to_string(), false),
                                spans[1],
                                Type::String,
                            ),
                        );
                        if let Some(main_block_id) = script_main_block_id {
                            call_with_block.set_parser_info(
                                "main_block_id".to_string(),
                                Expression::new(
                                    working_set,
                                    Expr::Int(main_block_id.get() as i64),
                                    spans[1],
                                    Type::Any,
                                ),
                            );
                        }
                        return Expression::new(
                            working_set,
                            Expr::Call(call_with_block),
                            Span::concat(spans),
                            Type::Any,
                        );
                    }
                } else {
                    working_set.error(ParseError::SourcedFileNotFound(filename, spans[1]));
                }
            }
            return Expression::new(
                working_set,
                Expr::Call(call),
                Span::concat(spans),
                Type::Any,
            );
        }
    }
    working_set.error(ParseError::UnknownState(
        "internal error: run statement unparsable".into(),
        Span::concat(spans),
    ));
    garbage(working_set, Span::concat(spans))
}

pub fn find_main_block_id_in_script(
    working_set: &StateWorkingSet<'_>,
    script_block: &Block,
) -> Option<BlockId> {
    script_block.pipelines.iter().find_map(|pipeline| {
        if pipeline.elements.len() != 1 {
            return None;
        }

        let expr = &pipeline.elements[0].expr;
        let Expr::Call(call) = &expr.expr else {
            return None;
        };
        let decl_name = working_set.get_decl(call.decl_id).name();
        if decl_name != "def" && decl_name != "export def" {
            return None;
        }

        let mut positional = call.positional_iter();
        let command_name = positional.next().and_then(Expression::as_string)?;
        if command_name != "main" {
            return None;
        }

        let _ = positional.next();
        positional.next().and_then(Expression::as_block)
    })
}

pub fn parse_run_expr(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    let lite_command = LiteCommand {
        parts: spans.to_vec(),
        pipe: None,
        redirection: None,
        comments: vec![],
        attribute_idx: vec![],
    };
    parse_run_expr_internal(working_set, spans, &lite_command)
}

pub fn parse_where_expr(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    trace!("parsing: where");

    if !spans.is_empty() && working_set.get_span_contents(spans[0]) != b"where" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'where' command".into(),
            Span::concat(spans),
        ));
        return garbage(working_set, Span::concat(spans));
    }

    if spans.len() < 2 {
        working_set.error(ParseError::MissingPositional(
            "row condition".into(),
            Span::concat(spans),
            "where <row_condition>".into(),
        ));
        return garbage(working_set, Span::concat(spans));
    }

    let call = match working_set.find_decl(b"where") {
        Some(decl_id) => {
            let ParsedInternalCall {
                call,
                output,
                call_kind,
            } = parse_internal_call(
                working_set,
                spans[0],
                &spans[1..],
                decl_id,
                ArgumentParsingLevel::Full,
                None,
            );

            if call_kind != CallKind::Valid {
                return Expression::new(working_set, Expr::Call(call), Span::concat(spans), output);
            }

            call
        }
        None => {
            working_set.error(ParseError::UnknownState(
                "internal error: 'where' declaration not found".into(),
                Span::concat(spans),
            ));
            return garbage(working_set, Span::concat(spans));
        }
    };

    Expression::new(
        working_set,
        Expr::Call(call),
        Span::concat(spans),
        Type::Any,
    )
}

pub fn parse_where(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    let expr = parse_where_expr(working_set, &lite_command.parts);
    let redirection = lite_command
        .redirection
        .as_ref()
        .map(|r| parse_redirection(working_set, r));

    let element = PipelineElement {
        pipe: None,
        expr,
        redirection,
    };

    Pipeline {
        elements: vec![element],
    }
}

#[cfg(feature = "plugin")]
pub fn parse_plugin_use(working_set: &mut StateWorkingSet, call: Box<Call>) -> Pipeline {
    use nu_protocol::{FromValue, PluginRegistryFile};

    #[allow(deprecated)]
    let cwd = working_set.get_cwd();

    if let Err(err) = (|| {
        let name = call
            .positional_iter()
            .next()
            .map(|expr| {
                eval_constant(working_set, expr)
                    .and_then(nu_protocol::Spanned::<String>::from_value)
                    .map_err(|err| err.wrap(working_set, call.head))
            })
            .expect("required positional should have been checked")?;

        let plugin_config = call
            .named_iter()
            .find(|(arg_name, _, _)| arg_name.item == "plugin-config")
            .map(|(_, _, expr)| {
                let expr = expr
                    .as_ref()
                    .expect("--plugin-config arg should have been checked already");
                eval_constant(working_set, expr)
                    .and_then(nu_protocol::Spanned::<String>::from_value)
                    .map_err(|err| err.wrap(working_set, call.head))
            })
            .transpose()?;

        let filename_query = {
            let path = nu_path::expand_path_with(&name.item, &cwd, true);
            path.to_str()
                .and_then(|path_str| {
                    find_in_dirs(path_str, working_set, &cwd, Some("NU_PLUGIN_DIRS"))
                })
                .map(|parser_path| parser_path.path_buf())
                .unwrap_or(path)
        };

        let plugin_config_path = if let Some(custom_path) = &plugin_config {
            find_in_dirs(&custom_path.item, working_set, &cwd, None).ok_or_else(|| {
                ParseError::FileNotFound(custom_path.item.clone(), custom_path.span)
            })?
        } else {
            ParserPath::RealPath(
                working_set
                    .permanent_state
                    .plugin_path
                    .as_ref()
                    .ok_or_else(|| ParseError::LabeledErrorWithHelp {
                        error: "Plugin registry file not set".into(),
                        label: "can't load plugin without registry file".into(),
                        span: call.head,
                        help:
                            "pass --plugin-config to `plugin use` when $nu.plugin-path is not set"
                                .into(),
                    })?
                    .to_owned(),
            )
        };

        let file = plugin_config_path.open(working_set).map_err(|err| {
            ParseError::LabeledError(
                "Plugin registry file can't be opened".into(),
                err.to_string(),
                plugin_config.as_ref().map(|p| p.span).unwrap_or(call.head),
            )
        })?;

        let contents = PluginRegistryFile::read_from(file, Some(call.head))
            .map_err(|err| err.wrap(working_set, call.head))?;

        let plugin_item = contents
            .plugins
            .iter()
            .find(|plugin| plugin.name == name.item || plugin.filename == filename_query)
            .ok_or_else(|| ParseError::PluginNotFound {
                name: name.item.clone(),
                name_span: name.span,
                plugin_config_span: plugin_config.as_ref().map(|p| p.span),
            })?;

        nu_plugin_engine::load_plugin_registry_item(working_set, plugin_item, Some(call.head))
            .map_err(|err| err.wrap(working_set, call.head))?;

        Ok(())
    })() {
        working_set.error(err);
    }

    let call_span = call.span();

    Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::Call(call),
        call_span,
        Type::Nothing,
    )])
}

pub fn find_dirs_var(working_set: &StateWorkingSet, var_name: &str) -> Option<VarId> {
    working_set
        .find_variable(format!("${var_name}").as_bytes())
        .filter(|var_id| working_set.get_variable(*var_id).const_val.is_some())
}

pub fn find_in_dirs(
    filename: &str,
    working_set: &StateWorkingSet,
    cwd: &str,
    dirs_var_name: Option<&str>,
) -> Option<ParserPath> {
    if is_windows_device_path(Path::new(&filename)) {
        return Some(ParserPath::RealPath(filename.into()));
    }

    pub fn find_in_dirs_with_id(
        filename: &str,
        working_set: &StateWorkingSet,
        cwd: &str,
        dirs_var_name: Option<&str>,
    ) -> Option<ParserPath> {
        let actual_cwd = working_set
            .files
            .current_working_directory()
            .unwrap_or(Path::new(cwd));

        if let Some(virtual_path) = working_set.find_virtual_path(filename) {
            return Some(ParserPath::from_virtual_path(
                working_set,
                filename,
                virtual_path,
            ));
        } else {
            let abs_virtual_filename = actual_cwd.join(filename);
            let abs_virtual_filename = abs_virtual_filename.to_string_lossy();

            if let Some(virtual_path) = working_set.find_virtual_path(&abs_virtual_filename) {
                return Some(ParserPath::from_virtual_path(
                    working_set,
                    &abs_virtual_filename,
                    virtual_path,
                ));
            }
        }

        if let Ok(p) = absolute_with(filename, actual_cwd)
            && p.exists()
        {
            return Some(ParserPath::RealPath(p));
        }

        let path = Path::new(filename);
        if !path.is_relative() {
            return None;
        }

        dirs_var_name
            .as_ref()
            .and_then(|dirs_var_name| find_dirs_var(working_set, dirs_var_name))
            .map(|var_id| working_set.get_variable(var_id))?
            .const_val
            .as_ref()?
            .as_list()
            .ok()?
            .iter()
            .map(|lib_dir| -> Option<PathBuf> {
                let dir = lib_dir.to_path().ok()?;
                let dir_abs = absolute_with(dir, actual_cwd).ok()?;
                let path = absolute_with(filename, dir_abs).ok()?;
                path.exists().then_some(path)
            })
            .find(Option::is_some)
            .flatten()
            .map(ParserPath::RealPath)
    }

    pub fn find_in_dirs_old(
        filename: &str,
        working_set: &StateWorkingSet,
        cwd: &str,
        dirs_env: Option<&str>,
    ) -> Option<PathBuf> {
        let actual_cwd = working_set
            .files
            .current_working_directory()
            .unwrap_or(Path::new(cwd));

        if let Ok(p) = absolute_with(filename, actual_cwd)
            && p.exists()
        {
            Some(p)
        } else {
            let path = Path::new(filename);

            if path.is_relative() {
                if let Some(lib_dirs) =
                    dirs_env.and_then(|dirs_env| working_set.get_env_var(dirs_env))
                {
                    if let Ok(dirs) = lib_dirs.as_list() {
                        for lib_dir in dirs {
                            if let Ok(dir) = lib_dir.to_path()
                                && let Ok(dir_abs) = absolute_with(dir, actual_cwd)
                                && let Ok(path) = absolute_with(filename, dir_abs)
                                && path.exists()
                            {
                                return Some(path);
                            }
                        }

                        None
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    find_in_dirs_with_id(filename, working_set, cwd, dirs_var_name).or_else(|| {
        find_in_dirs_old(filename, working_set, cwd, dirs_var_name).map(ParserPath::RealPath)
    })
}
