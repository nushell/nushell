//! Internals documentation: how the parser works and how to contribute.
//!
//! The chapters below are the Markdown files in `src/docs/`, rendered here so
//! they appear in `cargo doc` and so that their `rust` code blocks are
//! compiled and run by `cargo test --doc`. Start with [`architecture`].

#[doc = include_str!("README.md")]
pub mod index {}

#[doc = include_str!("01-architecture.md")]
pub mod architecture {}

#[doc = include_str!("02-lexer.md")]
pub mod lexer {}

#[doc = include_str!("03-streams-and-errors.md")]
pub mod streams_and_errors {}

#[doc = include_str!("04-blocks-and-pipelines.md")]
pub mod blocks_and_pipelines {}

#[doc = include_str!("05-statements-and-expressions.md")]
pub mod statements_and_expressions {}

#[doc = include_str!("06-values-and-literals.md")]
pub mod values_and_literals {}

#[doc = include_str!("07-signatures-types-and-patterns.md")]
pub mod signatures_types_and_patterns {}

#[doc = include_str!("08-ast-and-consumers.md")]
pub mod ast_and_consumers {}

#[doc = include_str!("09-testing-and-tools.md")]
pub mod testing_and_tools {}

#[doc = include_str!("10-contributing.md")]
pub mod contributing {}

#[doc = include_str!("11-traceability.md")]
pub mod traceability {}

#[doc = include_str!("how-to.md")]
pub mod how_to {}

#[doc = include_str!("../../examples/nufmt/README.md")]
pub mod nufmt_readme {}

#[doc = include_str!("nushell-integration-plan.md")]
pub mod nushell_integration_plan {}
