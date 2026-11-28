use nu_protocol::{
    CompileError, IntoSpanned, RegId, Span,
    ast::{Block, Expr, Pipeline, PipelineRedirection, RedirectionSource, RedirectionTarget},
    engine::StateWorkingSet,
    ir::{Instruction, IrBlock, RedirectMode, ScopeRegion},
};

mod builder;
mod call;
mod expression;
mod keyword;
mod operator;
mod redirect;

use builder::BlockBuilder;
use call::*;
use expression::compile_expression;
use operator::*;
use redirect::*;

const BLOCK_INPUT: RegId = RegId::new(0);

/// Compile Nushell pipeline abstract syntax tree (AST) to internal representation (IR) instructions
/// for evaluation.
pub fn compile(working_set: &StateWorkingSet, block: &Block) -> Result<IrBlock, CompileError> {
    let mut builder = BlockBuilder::new(block.span);

    let span = block.span.unwrap_or(Span::unknown());

    // Top-level: no scope region — `eval_ir_block` pushes this block's `scope_bindings`.
    compile_block(
        working_set,
        &mut builder,
        block,
        false,
        RedirectModes::caller(span),
        Some(BLOCK_INPUT),
        BLOCK_INPUT,
    )?;

    // A complete block has to end with a `return`
    builder.push(Instruction::Return { src: BLOCK_INPUT }.into_spanned(span))?;

    builder.finish()
}

/// Compiles a [`Block`] in-place into an IR block. This can be used in a nested manner, for example
/// by [`compile_if()`][keyword::compile_if], where the instructions for the blocks for the if/else
/// are inlined into the top-level IR block.
///
/// When `record_scope_region` is true and the block has parse-time `scope_bindings`, records a
/// [`ScopeRegion`] covering the inlined instructions so `scope` can see those locals by program
/// counter (keyword bodies never enter `eval_ir_block`). Pass `false` for the outer block from
/// [`compile`] (bindings activated at eval entry instead).
fn compile_block(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    block: &Block,
    record_scope_region: bool,
    redirect_modes: RedirectModes,
    in_reg: Option<RegId>,
    out_reg: RegId,
) -> Result<(), CompileError> {
    compile_block_with_input_at(
        working_set,
        builder,
        block,
        record_scope_region,
        redirect_modes,
        in_reg,
        out_reg,
        0,
    )
}

/// Like [`compile_block`], but the input goes to the pipeline at `input_pipeline` instead of the
/// first one, and the pipelines before it get no input. A block that uses `$in` gives its input
/// to the first statement that uses `$in`, so the statements before it can set things up.
#[allow(
    clippy::too_many_arguments,
    reason = "compile_block's arguments plus the input pipeline"
)]
fn compile_block_with_input_at(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    block: &Block,
    record_scope_region: bool,
    redirect_modes: RedirectModes,
    in_reg: Option<RegId>,
    out_reg: RegId,
    input_pipeline: usize,
) -> Result<(), CompileError> {
    let span = block.span.unwrap_or(Span::unknown());
    let region_start = builder.instructions.len();

    // The pipelines before the input's write their output to `out_reg`, so an input in that
    // register waits in one of its own until its pipeline comes up.
    let held_input = match in_reg {
        Some(reg) if input_pipeline > 0 && reg == out_reg => {
            let held = builder.next_register()?;
            builder.push(
                Instruction::Move {
                    dst: held,
                    src: reg,
                }
                .into_spanned(span),
            )?;
            Some(held)
        }
        _ => None,
    };

    // A closure parameter written as a pattern, as in `{|{$name}| ...}`, arrives in a hidden
    // variable. Bind the pattern's variables from it before the body runs.
    for (var_id, pattern) in &block.parameter_patterns {
        let reg = builder.next_register()?;
        builder.push(
            Instruction::LoadVariable {
                dst: reg,
                var_id: *var_id,
                preserve_origin: false,
            }
            .into_spanned(pattern.span),
        )?;
        builder.push(
            Instruction::Destructure {
                pattern: Box::new(pattern.pattern.clone()),
                src: reg,
            }
            .into_spanned(pattern.span),
        )?;
        builder.drop_reg(reg)?;
    }

    let mut redirect_modes = Some(redirect_modes);
    if !block.pipelines.is_empty() {
        let last_index = block.pipelines.len() - 1;
        for (index, pipeline) in block.pipelines.iter().enumerate() {
            // Put a held input back in `out_reg`, where a pipeline's input has to be: a call
            // moves `in_reg` to `out_reg` in `compile_expression` and again in `compile_call`.
            if index == input_pipeline
                && let Some(held) = held_input
            {
                builder.push(
                    Instruction::Move {
                        dst: out_reg,
                        src: held,
                    }
                    .into_spanned(span),
                )?;
            }
            compile_pipeline(
                working_set,
                builder,
                pipeline,
                span,
                // the redirect mode only applies to the last pipeline.
                if index == last_index {
                    redirect_modes
                        .take()
                        .expect("should only take redirect_modes once")
                } else {
                    RedirectModes::default()
                },
                // input is only passed to one pipeline, the first one unless told otherwise.
                if index == input_pipeline {
                    in_reg
                } else {
                    None
                },
                out_reg,
            )?;

            if index != last_index {
                // Explicitly drain the out reg after each non-final pipeline, because that's how
                // the semicolon functions.
                if builder.is_allocated(out_reg) {
                    builder.drain_statement(out_reg, span)?;
                }
                builder.load_empty(out_reg)?;
            }
        }
    } else if in_reg.is_none() {
        builder.load_empty(out_reg)?;
    }

    if record_scope_region && let Some(bindings) = &block.scope_bindings {
        let region_end = builder.instructions.len();
        // Empty inlined bodies produce no instructions; still record a zero-width region
        // is useless for PC matching, so only store non-empty ranges.
        if region_start < region_end {
            builder.scope_regions.push(ScopeRegion {
                start: region_start,
                end: region_end,
                bindings: bindings.clone(),
            });
        }
    }

    Ok(())
}

fn compile_pipeline(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    pipeline: &Pipeline,
    fallback_span: Span,
    redirect_modes: RedirectModes,
    in_reg: Option<RegId>,
    out_reg: RegId,
) -> Result<(), CompileError> {
    let mut iter = pipeline.elements.iter().peekable();
    let mut in_reg = in_reg;
    let mut redirect_modes = Some(redirect_modes);
    while let Some(element) = iter.next() {
        let span = element.pipe.unwrap_or(fallback_span);

        // We have to get the redirection mode from either the explicit redirection in the pipeline
        // element, or from the next expression if it's specified there. If this is the last
        // element, then it's from whatever is passed in as the mode to use.

        let next_redirect_modes = if let Some(next_element) = iter.peek() {
            let mut modes = redirect_modes_of_expression(working_set, &next_element.expr, span)?;

            // If there's a next element with no inherent redirection we always pipe out *unless*
            // this is a single redirection of stderr to pipe (e>|). `PipeRaw` is a plain pipe
            // too, so `e>|` leaves stdout alone for it as well.
            if matches!(
                element.redirection,
                Some(PipelineRedirection::Single {
                    source: RedirectionSource::Stderr,
                    target: RedirectionTarget::Pipe { .. }
                })
            ) {
                modes.out = modes
                    .out
                    .filter(|mode| !matches!(mode.item, RedirectMode::PipeRaw));
            } else if modes.out.is_none() {
                let pipe_span = next_element.pipe.unwrap_or(next_element.expr.span);
                modes.out = Some(RedirectMode::Pipe.into_spanned(pipe_span));
            }

            modes
        } else {
            redirect_modes
                .take()
                .expect("should only take redirect_modes once")
        };

        let spec_redirect_modes = match &element.redirection {
            Some(PipelineRedirection::Single { source, target }) => {
                let mode = redirection_target_to_mode(working_set, builder, target)?;
                match source {
                    RedirectionSource::Stdout => RedirectModes {
                        out: Some(mode),
                        err: None,
                    },
                    RedirectionSource::Stderr => RedirectModes {
                        out: None,
                        err: Some(mode),
                    },
                    RedirectionSource::StdoutAndStderr => RedirectModes {
                        out: Some(mode),
                        err: Some(mode),
                    },
                }
            }
            Some(PipelineRedirection::Separate { out, err }) => {
                // In this case, out and err must not both be Pipe
                assert!(
                    !matches!(
                        (out, err),
                        (
                            RedirectionTarget::Pipe { .. },
                            RedirectionTarget::Pipe { .. }
                        )
                    ),
                    "for Separate redirection, out and err targets must not both be Pipe"
                );
                let out = redirection_target_to_mode(working_set, builder, out)?;
                let err = redirection_target_to_mode(working_set, builder, err)?;
                RedirectModes {
                    out: Some(out),
                    err: Some(err),
                }
            }
            None => RedirectModes {
                out: None,
                err: None,
            },
        };

        let redirect_modes = RedirectModes {
            out: spec_redirect_modes.out.or(next_redirect_modes.out),
            err: spec_redirect_modes.err.or(next_redirect_modes.err),
        };

        compile_expression(
            working_set,
            builder,
            &element.expr,
            redirect_modes.clone(),
            in_reg,
            out_reg,
        )?;

        // Only clean up the redirection if current element is NOT
        // a nested eval expression, since this already cleans it.
        if !has_nested_eval_expr(&element.expr.expr) {
            // Clean up the redirection
            finish_redirection(builder, redirect_modes, out_reg)?;
        }

        // The next pipeline element takes input from this output
        in_reg = Some(out_reg);
    }
    Ok(())
}

fn has_nested_eval_expr(expr: &Expr) -> bool {
    is_subexpression(expr) || is_block_call(expr)
}

fn is_block_call(expr: &Expr) -> bool {
    match expr {
        Expr::Call(inner) => inner
            .arguments
            .iter()
            .any(|arg| matches!(arg.expr().map(|e| &e.expr), Some(Expr::Block(..)))),
        _ => false,
    }
}

fn is_subexpression(expr: &Expr) -> bool {
    match expr {
        Expr::FullCellPath(inner) => {
            matches!(&inner.head.expr, &Expr::Subexpression(..))
        }
        Expr::Subexpression(..) => true,
        _ => false,
    }
}
