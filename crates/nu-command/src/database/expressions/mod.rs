// Conversions between value and sqlparser objects
mod alias;
mod and;
mod as_nu;
mod field;
mod function;
mod or;
mod over;

use nu_protocol::engine::StateWorkingSet;

pub(crate) use alias::AliasExpr;
pub(crate) use and::AndExpr;
pub(crate) use as_nu::ExprAsNu;
pub(crate) use field::FieldExpr;
pub(crate) use function::FunctionExpr;
pub(crate) use or::OrExpr;
pub(crate) use over::OverExpr;

pub fn add_expressions_decls(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    // Series commands
    bind_command!(
        ExprAsNu,
        AliasExpr,
        AndExpr,
        FieldExpr,
        FunctionExpr,
        OrExpr,
        OverExpr
    );
}
