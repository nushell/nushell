mod col;
mod alias;

use nu_protocol::engine::StateWorkingSet;

use alias::AliasExpr;
use col::ColExpr;

pub fn add_expression_decls(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    // Series commands
    bind_command!(AliasExpr, ColExpr);
}