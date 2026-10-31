//! Tests of the winnow parser (`nu_parser::winnow::syntax`): its trees, errors and
//! highlighting shapes, with the fixtures and corpus beside this file. The front end's
//! tests (lowering into nu-protocol's AST, with the experimental option on) are in the
//! root crate's `tests/parsing/winnow*.rs`.

mod corpus;
mod examples;
mod fixtures;
mod language;
mod nufmt;
mod syntax;
mod traceability;
