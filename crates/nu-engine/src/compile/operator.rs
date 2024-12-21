use super::{compile_expression, BlockBuilder, CompileError, RedirectModes};
use crate::is_automatic_env_var;
use nu_protocol::{
    ast::{Assignment, Boolean, CellPath, Expr, Expression, Math, Operator, PathMember},
    engine::StateWorkingSet,
    ir::{Instruction, Literal},
    IntoSpanned, RegId, Span, Spanned, ENV_VARIABLE_ID,
};

pub(crate) fn compile_binary_op(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    lhs: &Expression,
    op: Spanned<Operator>,
    rhs: &Expression,
    span: Span,
    out_reg: RegId,
) -> Result<(), CompileError> {
    if let Operator::Assignment(assign_op) = op.item {
        if let Some(decomposed_op) = decompose_assignment(assign_op) {
            // Compiling an assignment that uses a binary op with the existing value
            compile_binary_op(
                working_set,
                builder,
                lhs,
                decomposed_op.into_spanned(op.span),
                rhs,
                span,
                out_reg,
            )?;
        } else {
            // Compiling a plain assignment, where the current left-hand side value doesn't matter
            compile_expression(
                working_set,
                builder,
                rhs,
                RedirectModes::value(rhs.span),
                None,
                out_reg,
            )?;
        }

        compile_assignment(working_set, builder, lhs, op.span, out_reg)?;

        // Load out_reg with Nothing, as that's the result of an assignment
        builder.load_literal(out_reg, Literal::Nothing.into_spanned(op.span))
    } else {
        // Not an assignment: just do the binary op
        let lhs_reg = out_reg;

        compile_expression(
            working_set,
            builder,
            lhs,
            RedirectModes::value(lhs.span),
            None,
            lhs_reg,
        )?;

        match op.item {
            // `and` / `or` are short-circuiting, and we can get by with one register and a branch
            Operator::Boolean(Boolean::And) => {
                let true_label = builder.label(None);
                builder.branch_if(lhs_reg, true_label, op.span)?;

                // If the branch was not taken it's false, so short circuit to load false
                let false_label = builder.label(None);
                builder.jump(false_label, op.span)?;

                builder.set_label(true_label, builder.here())?;
                compile_expression(
                    working_set,
                    builder,
                    rhs,
                    RedirectModes::value(rhs.span),
                    None,
                    lhs_reg,
                )?;

                let end_label = builder.label(None);
                builder.jump(end_label, op.span)?;

                // Consumed by `branch-if`, so we have to set it false again
                builder.set_label(false_label, builder.here())?;
                builder.load_literal(lhs_reg, Literal::Bool(false).into_spanned(lhs.span))?;

                builder.set_label(end_label, builder.here())?;
            }
            Operator::Boolean(Boolean::Or) => {
                let true_label = builder.label(None);
                builder.branch_if(lhs_reg, true_label, op.span)?;

                // If the branch was not taken it's false, so do the right-side expression
                compile_expression(
                    working_set,
                    builder,
                    rhs,
                    RedirectModes::value(rhs.span),
                    None,
                    lhs_reg,
                )?;

                let end_label = builder.label(None);
                builder.jump(end_label, op.span)?;

                // Consumed by `branch-if`, so we have to set it true again
                builder.set_label(true_label, builder.here())?;
                builder.load_literal(lhs_reg, Literal::Bool(true).into_spanned(lhs.span))?;

                builder.set_label(end_label, builder.here())?;
            }
            _ => {
                // Any other operator, via `binary-op`
                let rhs_reg = builder.next_register()?;

                compile_expression(
                    working_set,
                    builder,
                    rhs,
                    RedirectModes::value(rhs.span),
                    None,
                    rhs_reg,
                )?;

                builder.push(
                    Instruction::BinaryOp {
                        lhs_dst: lhs_reg,
                        op: op.item,
                        rhs: rhs_reg,
                    }
                    .into_spanned(op.span),
                )?;
            }
        }

        if lhs_reg != out_reg {
            builder.push(
                Instruction::Move {
                    dst: out_reg,
                    src: lhs_reg,
                }
                .into_spanned(op.span),
            )?;
        }

        builder.push(Instruction::Span { src_dst: out_reg }.into_spanned(span))?;

        Ok(())
    }
}

/// The equivalent plain operator to use for an assignment, if any
pub(crate) fn decompose_assignment(assignment: Assignment) -> Option<Operator> {
    match assignment {
        Assignment::Assign => None,
        Assignment::PlusAssign => Some(Operator::Math(Math::Plus)),
        Assignment::ConcatAssign => Some(Operator::Math(Math::Concat)),
        Assignment::MinusAssign => Some(Operator::Math(Math::Minus)),
        Assignment::MultiplyAssign => Some(Operator::Math(Math::Multiply)),
        Assignment::DivideAssign => Some(Operator::Math(Math::Divide)),
    }
}

/// Compile assignment of the value in a register to a left-hand expression
pub(crate) fn compile_assignment(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    lhs: &Expression,
    assignment_span: Span,
    rhs_reg: RegId,
) -> Result<(), CompileError> {
    match lhs.expr {
        Expr::Var(var_id) => {
            // Double check that the variable is supposed to be mutable
            if !working_set.get_variable(var_id).mutable {
                return Err(CompileError::AssignmentRequiresMutableVar { span: lhs.span });
            }

            builder.push(
                Instruction::StoreVariable {
                    var_id,
                    src: rhs_reg,
                }
                .into_spanned(assignment_span),
            )?;
            Ok(())
        }
        Expr::FullCellPath(ref path) => match (&path.head, &path.tail) {
            (
                Expression {
                    expr: Expr::Var(var_id),
                    ..
                },
                _,
            ) if *var_id == ENV_VARIABLE_ID => {
                // This will be an assignment to an environment variable.
                let Some(PathMember::String { val: key, .. }) = path.tail.first() else {
                    return Err(CompileError::CannotReplaceEnv { span: lhs.span });
                };

                // Some env vars can't be set by Nushell code.
                if is_automatic_env_var(key, true) {
                    return Err(CompileError::AutomaticEnvVarSetManually {
                        envvar_name: key.into(),
                        span: lhs.span,
                    });
                }

                let key_data = builder.data(key)?;

                let val_reg = if path.tail.len() > 1 {
                    // Get the current value of the head and first tail of the path, from env
                    let head_reg = builder.next_register()?;

                    // We could use compile_load_env, but this shares the key data...
                    // Always use optional, because it doesn't matter if it's already there
                    builder.push(
                        Instruction::LoadEnvOpt {
                            dst: head_reg,
                            key: key_data,
                        }
                        .into_spanned(lhs.span),
                    )?;

                    // Default to empty record so we can do further upserts
                    let default_label = builder.label(None);
                    let upsert_label = builder.label(None);
                    builder.branch_if_empty(head_reg, default_label, assignment_span)?;
                    builder.jump(upsert_label, assignment_span)?;

                    builder.set_label(default_label, builder.here())?;
                    builder.load_literal(
                        head_reg,
                        Literal::Record { capacity: 0 }.into_spanned(lhs.span),
                    )?;

                    // Do the upsert on the current value to incorporate rhs
                    builder.set_label(upsert_label, builder.here())?;
                    compile_upsert_cell_path(
                        builder,
                        (&path.tail[1..]).into_spanned(lhs.span),
                        head_reg,
                        rhs_reg,
                        assignment_span,
                    )?;

                    head_reg
                } else {
                    // Path has only one tail, so we don't need the current value to do an upsert,
                    // just set it directly to rhs
                    rhs_reg
                };

                // Finally, store the modified env variable
                builder.push(
                    Instruction::StoreEnv {
                        key: key_data,
                        src: val_reg,
                    }
                    .into_spanned(assignment_span),
                )?;
                Ok(())
            }
            (_, tail) if tail.is_empty() => {
                // If the path tail is empty, we can really just treat this as if it were an
                // assignment to the head
                compile_assignment(working_set, builder, &path.head, assignment_span, rhs_reg)
            }
            _ => {
                // Just a normal assignment to some path
                let head_reg = builder.next_register()?;

                // Compile getting current value of the head expression
                compile_expression(
                    working_set,
                    builder,
                    &path.head,
                    RedirectModes::value(path.head.span),
                    None,
                    head_reg,
                )?;

                // Upsert the tail of the path into the old value of the head expression
                compile_upsert_cell_path(
                    builder,
                    path.tail.as_slice().into_spanned(lhs.span),
                    head_reg,
                    rhs_reg,
                    assignment_span,
                )?;

                // Now compile the assignment of the updated value to the head
                compile_assignment(working_set, builder, &path.head, assignment_span, head_reg)
            }
        },
        Expr::Garbage => Err(CompileError::Garbage { span: lhs.span }),
        _ => Err(CompileError::AssignmentRequiresVar { span: lhs.span }),
    }
}

/// Compile an upsert-cell-path instruction, with known literal members
pub(crate) fn compile_upsert_cell_path(
    builder: &mut BlockBuilder,
    members: Spanned<&[PathMember]>,
    src_dst: RegId,
    new_value: RegId,
    span: Span,
) -> Result<(), CompileError> {
    let path_reg = builder.literal(
        Literal::CellPath(
            CellPath {
                members: members.item.to_vec(),
            }
            .into(),
        )
        .into_spanned(members.span),
    )?;
    builder.push(
        Instruction::UpsertCellPath {
            src_dst,
            path: path_reg,
            new_value,
        }
        .into_spanned(span),
    )?;
    Ok(())
}

/// Compile the correct sequence to get an environment variable + follow a path on it
pub(crate) fn compile_load_env(
    builder: &mut BlockBuilder,
    span: Span,
    path: &[PathMember],
    out_reg: RegId,
) -> Result<(), CompileError> {
    if path.is_empty() {
        builder.push(
            Instruction::LoadVariable {
                dst: out_reg,
                var_id: ENV_VARIABLE_ID,
            }
            .into_spanned(span),
        )?;
    } else {
        let (key, optional) = match &path[0] {
            PathMember::String { val, optional, .. } => (builder.data(val)?, *optional),
            PathMember::Int { span, .. } => {
                return Err(CompileError::AccessEnvByInt { span: *span })
            }
        };
        let tail = &path[1..];

        if optional {
            builder.push(Instruction::LoadEnvOpt { dst: out_reg, key }.into_spanned(span))?;
        } else {
            builder.push(Instruction::LoadEnv { dst: out_reg, key }.into_spanned(span))?;
        }

        if !tail.is_empty() {
            let path = builder.literal(
                Literal::CellPath(Box::new(CellPath {
                    members: tail.to_vec(),
                }))
                .into_spanned(span),
            )?;
            builder.push(
                Instruction::FollowCellPath {
                    src_dst: out_reg,
                    path,
                }
                .into_spanned(span),
            )?;
        }
    }
    Ok(())
}
