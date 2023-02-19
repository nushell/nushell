use crate::engine::{EngineState, Stack};
use crate::{
    ast::{Argument, Call, Expr, Expression},
    engine::Command,
    BlockId, Example, ShellError, Signature,
};
use crate::{PipelineData, Type};
use std::path::PathBuf;

#[derive(Clone)]
pub enum WrappedCall {
    Call(Box<Call>),
    ExternalCall(Box<Expression>, Vec<Expression>, bool), // head, args, is_subexpression
}

#[derive(Clone)]
pub struct Alias {
    pub name: String,
    pub command: Option<Box<dyn Command>>, // None if external call
    pub wrapped_call: Expression,
}

impl Alias {
    pub fn unwrap_call_from_alias(&self, call: &Call) -> Result<Expression, ShellError> {
        match &self.wrapped_call {
            Expression {
                expr: Expr::Call(wrapped_call),
                ty,
                span,
                custom_completion,
            } => {
                let mut final_call = wrapped_call.clone();

                for arg in &call.arguments {
                    final_call.arguments.push(arg.clone());
                }

                Ok(Expression {
                    expr: Expr::Call(final_call),
                    span: *span,
                    ty: ty.clone(),
                    custom_completion: *custom_completion,
                })
            }
            Expression {
                expr: Expr::ExternalCall(head, args, is_subexpression),
                ty,
                span,
                custom_completion,
            } => {
                let mut final_args = args.clone();

                // logic taken from KnownExternal::run()
                for arg in &call.arguments {
                    match arg {
                        Argument::Positional(positional) => final_args.push(positional.clone()),
                        Argument::Named(named) => {
                            if let Some(short) = &named.1 {
                                final_args.push(Expression {
                                    expr: Expr::String(format!("-{}", short.item)),
                                    span: named.0.span,
                                    ty: Type::String,
                                    custom_completion: None,
                                });
                            } else {
                                final_args.push(Expression {
                                    expr: Expr::String(format!("--{}", named.0.item)),
                                    span: named.0.span,
                                    ty: Type::String,
                                    custom_completion: None,
                                });
                            }
                            if let Some(arg) = &named.2 {
                                final_args.push(arg.clone());
                            }
                        }
                        Argument::Unknown(unknown) => final_args.push(unknown.clone()),
                    }
                }

                Ok(Expression {
                    expr: Expr::ExternalCall(head.clone(), final_args, *is_subexpression),
                    span: *span,
                    ty: ty.clone(),
                    custom_completion: *custom_completion,
                })
            }
            _ => Err(ShellError::NushellFailedSpannedHelp(
                "Alias aliases unsupported expression".to_string(),
                format!("{:?} not supported", self.wrapped_call.expr),
                self.wrapped_call.span,
                "Only call to a custom or external command is supported.".to_string(),
            )),
        }
    }
}

impl Command for Alias {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        if let Some(cmd) = &self.command {
            cmd.signature()
        } else {
            Signature::new(&self.name).allows_unknown_args()
        }
    }

    fn usage(&self) -> &str {
        if let Some(cmd) = &self.command {
            cmd.usage()
        } else {
            "This alias wraps an unknown external command."
        }
    }

    fn extra_usage(&self) -> &str {
        if let Some(cmd) = &self.command {
            cmd.extra_usage()
        } else {
            ""
        }
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::NushellFailedSpanned(
            "Can't run alias directly. Unwrap it first".to_string(),
            "originates from here".to_string(),
            call.head,
        ))
    }

    fn examples(&self) -> Vec<Example> {
        if let Some(cmd) = &self.command {
            cmd.examples()
        } else {
            vec![]
        }
    }

    fn is_builtin(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_builtin()
        } else {
            false
        }
    }

    fn is_known_external(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_known_external()
        } else {
            false
        }
    }

    fn is_alias(&self) -> bool {
        true
    }

    fn as_alias(&self) -> Option<&Alias> {
        Some(self)
    }

    fn is_custom_command(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_custom_command()
        } else if self.get_block_id().is_some() {
            true
        } else {
            self.is_known_external()
        }
    }

    fn is_sub(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_sub()
        } else {
            self.name().contains(' ')
        }
    }

    fn is_parser_keyword(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_parser_keyword()
        } else {
            false
        }
    }

    fn is_plugin(&self) -> Option<(&PathBuf, &Option<PathBuf>)> {
        if let Some(cmd) = &self.command {
            cmd.is_plugin()
        } else {
            None
        }
    }

    fn get_block_id(&self) -> Option<BlockId> {
        if let Some(cmd) = &self.command {
            cmd.get_block_id()
        } else {
            None
        }
    }

    fn search_terms(&self) -> Vec<&str> {
        if let Some(cmd) = &self.command {
            cmd.search_terms()
        } else {
            vec![]
        }
    }
}
