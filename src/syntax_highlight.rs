use std::{cell::RefCell, rc::Rc};

use crate::{Block, Expr, Expression, ParserState, ParserWorkingSet, Statement};

struct Highlighter {
    parser_state: Rc<RefCell<ParserState>>,
}

impl Highlighter {
    fn syntax_highlight(&self, input: &[u8]) {
        let block = {
            let parser_state = self.parser_state.borrow();
            let mut working_set = ParserWorkingSet::new(&*parser_state);
            let (block, _) = working_set.parse_source(input, false);

            block
        };

        // let (block, _) = working_set.parse_source(input, false);

        // for stmt in &block.stmts {
        //     match stmt {
        //         Statement::Expression(expr) => {

        //         }
        //     }
        // }
        // No merge at the end because this parse is speculative
    }

    fn highlight_expression(expression: &Expression) {
        // match &expression.expr {
        //     Expr::BinaryOp()
        // }
    }
}
