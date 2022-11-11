use nu_parser::ParseError;
use nu_parser::*;
use nu_protocol::{
    ast::{Expr, Expression},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Signature, SyntaxShape,
};

#[cfg(test)]
#[derive(Clone)]
pub struct Let;

#[cfg(test)]
impl Command for Let {
    fn name(&self) -> &str {
        "let"
    }

    fn usage(&self) -> &str {
        "Create a variable and give it a value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("let")
            .required("var_name", SyntaxShape::VarWithOptType, "variable name")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "equals sign followed by value",
            )
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &nu_protocol::ast::Call,
        _input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        todo!()
    }
}

#[test]
pub fn parse_int() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"3", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert!(matches!(
        expressions[0],
        Expression {
            expr: Expr::Int(3),
            ..
        }
    ))
}

#[test]
pub fn parse_binary_with_hex_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0x[13]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0x13]))
}

#[test]
pub fn parse_binary_with_incomplete_hex_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0x[3]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0x03]))
}

#[test]
pub fn parse_binary_with_binary_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0b[1010 1000]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0b10101000]))
}

#[test]
pub fn parse_binary_with_incomplete_binary_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0b[10]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0b00000010]))
}

#[test]
pub fn parse_binary_with_octal_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0o[250]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0o250]))
}

#[test]
pub fn parse_binary_with_incomplete_octal_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0o[2]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(expressions[0].expr, Expr::Binary(vec![0o2]))
}

#[test]
pub fn parse_binary_with_invalid_octal_format() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"0b[90]", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert!(!matches!(&expressions[0].expr, Expr::Binary(_)))
}

#[test]
pub fn parse_binary_with_multi_byte_char() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    // found using fuzzing, Rust can panic if you slice into this string
    let contents = b"0x[\xEF\xBF\xBD]";
    let (block, err) = parse(&mut working_set, None, contents, true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert!(!matches!(&expressions[0].expr, Expr::Binary(_)))
}

#[test]
pub fn parse_string() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(&mut working_set, None, b"\"hello nushell\"", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(
        expressions[0].expr,
        Expr::String("hello nushell".to_string())
    )
}

#[test]
pub fn parse_escaped_string() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let (block, err) = parse(
        &mut working_set,
        None,
        b"\"hello \\u006e\\u0075\\u0073hell\"",
        true,
        &[],
    );

    assert!(err.is_none());
    assert!(block.len() == 1);
    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert_eq!(
        expressions[0].expr,
        Expr::String("hello nushell".to_string())
    )
}

#[test]
pub fn parse_call() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (block, err) = parse(&mut working_set, None, b"foo", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);

    let expressions = &block[0];
    assert_eq!(expressions.len(), 1);

    if let Expression {
        expr: Expr::Call(call),
        ..
    } = &expressions[0]
    {
        assert_eq!(call.decl_id, 0);
    }
}

#[test]
pub fn parse_call_missing_flag_arg() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (_, err) = parse(&mut working_set, None, b"foo --jazz", true, &[]);
    assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
}

#[test]
pub fn parse_call_missing_short_flag_arg() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());

    let (_, err) = parse(&mut working_set, None, b"foo -j", true, &[]);
    assert!(matches!(err, Some(ParseError::MissingFlagParam(..))));
}

#[test]
pub fn parse_call_too_many_shortflag_args() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo")
        .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
        .named("--math", SyntaxShape::Int, "math!!", Some('m'));
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo -mj", true, &[]);
    assert!(matches!(
        err,
        Some(ParseError::ShortFlagBatchCantTakeArg(..))
    ));
}

#[test]
pub fn parse_call_unknown_shorthand() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo -mj", true, &[]);
    assert!(matches!(err, Some(ParseError::UnknownFlag(..))));
}

#[test]
pub fn parse_call_extra_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").switch("--jazz", "jazz!!", Some('j'));
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo -j 100", true, &[]);
    assert!(matches!(err, Some(ParseError::ExtraPositional(..))));
}

#[test]
pub fn parse_call_missing_req_positional() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required("jazz", SyntaxShape::Int, "jazz!!");
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo", true, &[]);
    assert!(matches!(err, Some(ParseError::MissingPositional(..))));
}

#[test]
pub fn parse_call_missing_req_flag() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let sig = Signature::build("foo").required_named("--jazz", SyntaxShape::Int, "jazz!!", None);
    working_set.add_decl(sig.predeclare());
    let (_, err) = parse(&mut working_set, None, b"foo", true, &[]);
    assert!(matches!(err, Some(ParseError::MissingRequiredFlag(..))));
}

#[test]
fn test_nothing_comparisson_eq() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let (block, err) = parse(&mut working_set, None, b"2 == $nothing", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);

    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert!(matches!(
        &expressions[0],
        Expression {
            expr: Expr::BinaryOp(..),
            ..
        }
    ))
}

#[test]
fn test_nothing_comparisson_neq() {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);
    let (block, err) = parse(&mut working_set, None, b"2 != $nothing", true, &[]);

    assert!(err.is_none());
    assert!(block.len() == 1);

    let expressions = &block[0];
    assert!(expressions.len() == 1);
    assert!(matches!(
        &expressions[0],
        Expression {
            expr: Expr::BinaryOp(..),
            ..
        }
    ))
}

mod range {
    use super::*;
    use nu_protocol::ast::{RangeInclusion, RangeOperator};

    #[test]
    fn parse_inclusive_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"0..10", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
                    Some(_),
                    RangeOperator {
                        inclusion: RangeInclusion::Inclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn parse_exclusive_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"0..<10", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
                    Some(_),
                    RangeOperator {
                        inclusion: RangeInclusion::RightExclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn parse_reverse_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"10..0", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
                    Some(_),
                    RangeOperator {
                        inclusion: RangeInclusion::Inclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn parse_subexpression_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"(3 - 3)..<(8 + 2)", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
                    Some(_),
                    RangeOperator {
                        inclusion: RangeInclusion::RightExclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn parse_variable_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        working_set.add_decl(Box::new(Let));

        let (block, err) = parse(&mut working_set, None, b"let a = 2; $a..10", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 2);

        let expressions = &block[1];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
                    Some(_),
                    RangeOperator {
                        inclusion: RangeInclusion::Inclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn parse_subexpression_variable_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        working_set.add_decl(Box::new(Let));

        let (block, err) = parse(
            &mut working_set,
            None,
            b"let a = 2; $a..<($a + 10)",
            true,
            &[],
        );

        assert!(err.is_none());
        assert!(block.len() == 2);

        let expressions = &block[1];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
                    Some(_),
                    RangeOperator {
                        inclusion: RangeInclusion::RightExclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn parse_right_unbounded_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"0..", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
                    None,
                    RangeOperator {
                        inclusion: RangeInclusion::Inclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn parse_left_unbounded_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"..10", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    None,
                    None,
                    Some(_),
                    RangeOperator {
                        inclusion: RangeInclusion::Inclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn parse_negative_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"-10..-3", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    None,
                    Some(_),
                    RangeOperator {
                        inclusion: RangeInclusion::Inclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn parse_float_range() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (block, err) = parse(&mut working_set, None, b"2.0..4.0..10.0", true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 1);
        assert!(matches!(
            expressions[0],
            Expression {
                expr: Expr::Range(
                    Some(_),
                    Some(_),
                    Some(_),
                    RangeOperator {
                        inclusion: RangeInclusion::Inclusive,
                        ..
                    }
                ),
                ..
            }
        ))
    }

    #[test]
    fn bad_parse_does_crash() {
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);

        let (_, err) = parse(&mut working_set, None, b"(0)..\"a\"", true, &[]);

        assert!(err.is_some());
    }
}

#[cfg(test)]
mod input_types {
    use super::*;
    use nu_protocol::{ast::Argument, Category, Type};

    #[derive(Clone)]
    pub struct LsTest;

    impl Command for LsTest {
        fn name(&self) -> &str {
            "ls"
        }

        fn usage(&self) -> &str {
            "Mock ls command"
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name()).category(Category::Default)
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &nu_protocol::ast::Call,
            _input: nu_protocol::PipelineData,
        ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct GroupBy;

    impl Command for GroupBy {
        fn name(&self) -> &str {
            "group-by"
        }

        fn usage(&self) -> &str {
            "Mock group-by command"
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .required("column", SyntaxShape::String, "column name")
                .category(Category::Default)
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &nu_protocol::ast::Call,
            _input: nu_protocol::PipelineData,
        ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct ToCustom;

    impl Command for ToCustom {
        fn name(&self) -> &str {
            "to-custom"
        }

        fn usage(&self) -> &str {
            "Mock converter command"
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .input_type(Type::Any)
                .output_type(Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &nu_protocol::ast::Call,
            _input: nu_protocol::PipelineData,
        ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct GroupByCustom;

    impl Command for GroupByCustom {
        fn name(&self) -> &str {
            "group-by"
        }

        fn usage(&self) -> &str {
            "Mock custom group-by command"
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .required("column", SyntaxShape::String, "column name")
                .required("other", SyntaxShape::String, "other value")
                .input_type(Type::Custom("custom".into()))
                .output_type(Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &nu_protocol::ast::Call,
            _input: nu_protocol::PipelineData,
        ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct AggCustom;

    impl Command for AggCustom {
        fn name(&self) -> &str {
            "agg"
        }

        fn usage(&self) -> &str {
            "Mock custom agg command"
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .required("operation", SyntaxShape::String, "operation")
                .input_type(Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &nu_protocol::ast::Call,
            _input: nu_protocol::PipelineData,
        ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct AggMin;

    impl Command for AggMin {
        fn name(&self) -> &str {
            "min"
        }

        fn usage(&self) -> &str {
            "Mock custom min command"
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name()).category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &nu_protocol::ast::Call,
            _input: nu_protocol::PipelineData,
        ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct WithColumn;

    impl Command for WithColumn {
        fn name(&self) -> &str {
            "with-column"
        }

        fn usage(&self) -> &str {
            "Mock custom with-column command"
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .rest("operation", SyntaxShape::Any, "operation")
                .input_type(Type::Custom("custom".into()))
                .output_type(Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &nu_protocol::ast::Call,
            _input: nu_protocol::PipelineData,
        ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct Collect;

    impl Command for Collect {
        fn name(&self) -> &str {
            "collect"
        }

        fn usage(&self) -> &str {
            "Mock custom collect command"
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build(self.name())
                .input_type(Type::Custom("custom".into()))
                .output_type(Type::Custom("custom".into()))
                .category(Category::Custom("custom".into()))
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &nu_protocol::ast::Call,
            _input: nu_protocol::PipelineData,
        ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
            todo!()
        }
    }

    #[derive(Clone)]
    pub struct IfMocked;

    impl Command for IfMocked {
        fn name(&self) -> &str {
            "if"
        }

        fn usage(&self) -> &str {
            "Mock if command"
        }

        fn signature(&self) -> nu_protocol::Signature {
            Signature::build("if")
                .required("cond", SyntaxShape::Expression, "condition to check")
                .required(
                    "then_block",
                    SyntaxShape::Block,
                    "block to run if check succeeds",
                )
                .optional(
                    "else_expression",
                    SyntaxShape::Keyword(b"else".to_vec(), Box::new(SyntaxShape::Expression)),
                    "expression or block to run if check fails",
                )
                .category(Category::Core)
        }

        fn run(
            &self,
            _engine_state: &EngineState,
            _stack: &mut Stack,
            _call: &nu_protocol::ast::Call,
            _input: nu_protocol::PipelineData,
        ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
            todo!()
        }
    }

    fn add_declations(engine_state: &mut EngineState) {
        let delta = {
            let mut working_set = StateWorkingSet::new(engine_state);
            working_set.add_decl(Box::new(Let));
            working_set.add_decl(Box::new(AggCustom));
            working_set.add_decl(Box::new(GroupByCustom));
            working_set.add_decl(Box::new(GroupBy));
            working_set.add_decl(Box::new(LsTest));
            working_set.add_decl(Box::new(ToCustom));
            working_set.add_decl(Box::new(AggMin));
            working_set.add_decl(Box::new(Collect));
            working_set.add_decl(Box::new(WithColumn));
            working_set.add_decl(Box::new(IfMocked));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");
    }

    #[test]
    fn call_types_test() {
        let mut engine_state = EngineState::new();
        add_declations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let input = r#"ls | to-custom | group-by name other"#;

        let (block, err) = parse(&mut working_set, None, input.as_bytes(), true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 3);

        match &expressions[0].expr {
            Expr::Call(call) => {
                let expected_id = working_set
                    .find_decl(b"ls", &Type::Any)
                    .expect("Error merging delta");
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }

        match &expressions[1].expr {
            Expr::Call(call) => {
                let expected_id = working_set.find_decl(b"to-custom", &Type::Any).unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }

        match &expressions[2].expr {
            Expr::Call(call) => {
                let expected_id = working_set
                    .find_decl(b"group-by", &Type::Custom("custom".into()))
                    .unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn storing_variable_test() {
        let mut engine_state = EngineState::new();
        add_declations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let input =
            r#"let a = (ls | to-custom | group-by name other); let b = (1+3); $a | agg sum"#;

        let (block, err) = parse(&mut working_set, None, input.as_bytes(), true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 3);

        let expressions = &block[2];
        match &expressions[1].expr {
            Expr::Call(call) => {
                let expected_id = working_set
                    .find_decl(b"agg", &Type::Custom("custom".into()))
                    .unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn stored_variable_operation_test() {
        let mut engine_state = EngineState::new();
        add_declations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let input = r#"let a = (ls | to-custom | group-by name other); ($a + $a) | agg sum"#;

        let (block, err) = parse(&mut working_set, None, input.as_bytes(), true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 2);

        let expressions = &block[1];
        match &expressions[1].expr {
            Expr::Call(call) => {
                let expected_id = working_set
                    .find_decl(b"agg", &Type::Custom("custom".into()))
                    .unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn multiple_stored_variable_test() {
        let mut engine_state = EngineState::new();
        add_declations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let input = r#"
        let a = (ls | to-custom | group-by name other); [1 2 3] | to-custom; [1 2 3] | to-custom"#;

        let (block, err) = parse(&mut working_set, None, input.as_bytes(), true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 3);

        let expressions = &block[1];
        match &expressions[1].expr {
            Expr::Call(call) => {
                let expected_id = working_set.find_decl(b"to-custom", &Type::Any).unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }

        let expressions = &block[2];
        match &expressions[1].expr {
            Expr::Call(call) => {
                let expected_id = working_set.find_decl(b"to-custom", &Type::Any).unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn call_non_custom_types_test() {
        let mut engine_state = EngineState::new();
        add_declations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let input = r#"ls | group-by name"#;

        let (block, err) = parse(&mut working_set, None, input.as_bytes(), true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        assert!(expressions.len() == 2);

        match &expressions[0].expr {
            Expr::Call(call) => {
                let expected_id = working_set.find_decl(b"ls", &Type::Any).unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }

        match &expressions[1].expr {
            Expr::Call(call) => {
                let expected_id = working_set.find_decl(b"group-by", &Type::Any).unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn nested_operations_test() {
        let mut engine_state = EngineState::new();
        add_declations(&mut engine_state);

        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let input = r#"ls | to-custom | group-by name other | agg ("b" | min)"#;
            let (block, _) = parse(&mut working_set, None, input.as_bytes(), true, &[]);

            (block, working_set.render())
        };

        engine_state.merge_delta(delta).unwrap();

        let expressions = &block[0];
        match &expressions[3].expr {
            Expr::Call(call) => {
                let arg = &call.arguments[0];
                match arg {
                    Argument::Positional(a) => match &a.expr {
                        Expr::FullCellPath(path) => match &path.head.expr {
                            Expr::Subexpression(id) => {
                                let block = engine_state.get_block(*id);

                                let expressions = &block[0];
                                assert!(expressions.len() == 2);

                                match &expressions[1].expr {
                                    Expr::Call(call) => {
                                        let working_set = StateWorkingSet::new(&engine_state);
                                        let expected_id =
                                            working_set.find_decl(b"min", &Type::Any).unwrap();
                                        assert_eq!(call.decl_id, expected_id)
                                    }
                                    _ => panic!("Expected expression Call not found"),
                                }
                            }
                            _ => panic!("Expected Subexpression not found"),
                        },
                        _ => panic!("Expected FullCellPath not found"),
                    },
                    _ => panic!("Expected Argument Positional not found"),
                }
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn call_with_list_test() {
        let mut engine_state = EngineState::new();
        add_declations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let input = r#"[[a b]; [1 2] [3 4]] | to-custom | with-column [ ("a" | min) ("b" | min) ] | collect"#;

        let (block, err) = parse(&mut working_set, None, input.as_bytes(), true, &[]);

        assert!(err.is_none());
        assert!(block.len() == 1);

        let expressions = &block[0];
        match &expressions[2].expr {
            Expr::Call(call) => {
                let expected_id = working_set
                    .find_decl(b"with-column", &Type::Custom("custom".into()))
                    .unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }

        match &expressions[3].expr {
            Expr::Call(call) => {
                let expected_id = working_set
                    .find_decl(b"collect", &Type::Custom("custom".into()))
                    .unwrap();
                assert_eq!(call.decl_id, expected_id)
            }
            _ => panic!("Expected expression Call not found"),
        }
    }

    #[test]
    fn operations_within_blocks_test() {
        let mut engine_state = EngineState::new();
        add_declations(&mut engine_state);

        let mut working_set = StateWorkingSet::new(&engine_state);
        let inputs = vec![
            r#"let a = 'b'; ($a == 'b') || ($a == 'b')"#,
            r#"let a = 'b'; ($a == 'b') || ($a == 'b') && ($a == 'b')"#,
            r#"let a = 1; ($a == 1) || ($a == 2) && ($a == 3)"#,
            r#"let a = 'b'; if ($a == 'b') || ($a == 'b') { true } else { false }"#,
            r#"let a = 1; if ($a == 1) || ($a > 0) { true } else { false }"#,
        ];

        for input in inputs {
            let (block, err) = parse(&mut working_set, None, input.as_bytes(), true, &[]);

            assert!(err.is_none(), "testing: {}", input);
            assert!(block.len() == 2, "testing: {}", input);
        }
    }
}
