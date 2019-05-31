// auto-generated: "lalrpop 0.17.0"
// sha256: c7eac268e354044ccb73aea4829c2dbd02ae11ce74a7dc33b74944ec862f9
#![allow(unused)]
use std::str::FromStr;
use crate::parser::ast::*;
use crate::prelude::*;
use crate::parser::lexer::{SpannedToken, Token};
#[allow(unused_extern_crates)]
extern crate lalrpop_util as __lalrpop_util;
#[allow(unused_imports)]
use self::__lalrpop_util::state_machine as __state_machine;

#[cfg_attr(rustfmt, rustfmt_skip)]
mod __parse__Pipeline {
    #![allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables, unused_imports, unused_parens)]

    use std::str::FromStr;
    use crate::parser::ast::*;
    use crate::prelude::*;
    use crate::parser::lexer::{SpannedToken, Token};
    #[allow(unused_extern_crates)]
    extern crate lalrpop_util as __lalrpop_util;
    #[allow(unused_imports)]
    use self::__lalrpop_util::state_machine as __state_machine;
    use super::__ToTriple;
    #[allow(dead_code)]
    pub enum __Symbol<'input>
     {
        Variant0(SpannedToken<'input>),
        Variant1(::std::vec::Vec<SpannedToken<'input>>),
        Variant2(String),
        Variant3(::std::vec::Vec<String>),
        Variant4(ParsedCommand),
        Variant5(::std::vec::Vec<ParsedCommand>),
        Variant6(Expression),
        Variant7(BarePath),
        Variant8(::std::vec::Vec<Expression>),
        Variant9(Flag),
        Variant10(i64),
        Variant11(Operator),
        Variant12(Pipeline),
        Variant13(Variable),
    }
    const __ACTION: &'static [i8] = &[
        // State 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 1
        0, 21, 22, 0, 23, 24, 0, 0, 0, 0, 0, 0, 5, 25, 0, 26, 0, 27, 0, 28, -19, 0,
        // State 2
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 30, 0,
        // State 3
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 4
        -14, -14, -14, -14, -14, -14, -14, -14, -14, -14, -14, 32, -14, -14, 0, -14, 0, -14, 0, -14, -14, -14,
        // State 5
        -51, -51, -51, -51, -51, -51, -51, -51, -51, -51, -51, -51, -51, -51, 0, -51, 0, -51, 0, -51, -51, -51,
        // State 6
        -44, -44, -44, -44, -44, -44, -44, -44, -44, -44, -44, 0, -44, -44, 0, -44, 0, -44, 0, -44, -44, -44,
        // State 7
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -21, 0,
        // State 8
        -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, -52, 0, -52, 0, -52, 0, -52, -52, -52,
        // State 9
        34, -24, -24, 0, -24, -24, 35, 36, 37, 38, 39, 0, -24, -24, 0, -24, 0, -24, 0, -24, -24, 0,
        // State 10
        0, 21, 22, 0, 23, 24, 0, 0, 0, 0, 0, 0, 5, 25, 0, 26, 0, 27, 0, 28, -20, 0,
        // State 11
        -45, -45, -45, -45, -45, -45, -45, -45, -45, -45, -45, 0, -45, -45, 0, -45, 0, -45, 0, -45, -45, -45,
        // State 12
        -30, -30, -30, -30, -30, -30, -30, -30, -30, -30, -30, -30, -30, -30, 0, -30, 0, -30, 0, -30, -30, -30,
        // State 13
        -13, -13, -13, -13, -13, -13, -13, -13, -13, -13, -13, -13, -13, -13, 0, -13, 0, -13, 0, -13, -13, -13,
        // State 14
        -12, -12, -12, -12, -12, -12, -12, -12, -12, -12, -12, -12, -12, -12, 0, -12, 0, -12, 0, -12, -12, -12,
        // State 15
        -22, -22, -22, -22, -22, -22, -22, -22, -22, -22, -22, 0, -22, -22, 0, -22, 0, -22, 0, -22, -22, -22,
        // State 16
        -23, -23, -23, -23, -23, -23, -23, -23, -23, -23, -23, 0, -23, -23, 0, -23, 0, -23, 0, -23, -23, -23,
        // State 17
        -29, -29, -29, -29, -29, -29, -29, -29, -29, -29, -29, -29, -29, -29, 0, -29, 0, -29, 0, -29, -29, -29,
        // State 18
        -31, -31, -31, -31, -31, -31, -31, -31, -31, -31, -31, -31, -31, -31, 0, -31, 0, -31, 0, -31, -31, -31,
        // State 19
        -43, -43, -43, -43, -43, -43, -43, -43, -43, -43, -43, 42, -43, -43, 0, -43, 0, -43, 0, -43, -43, -43,
        // State 20
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 43, 0, 0, 0,
        // State 21
        0, 21, 22, 0, 23, 24, 0, 0, 0, 0, 0, 0, 5, 25, 0, 26, 0, 27, 0, 28, 0, 0,
        // State 22
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 23
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 24
        -49, -49, -49, -49, -49, -49, -49, -49, -49, -49, -49, -49, -49, -49, 0, -49, 0, -49, 0, -49, -49, -49,
        // State 25
        -28, -28, -28, -28, -28, -28, -28, -28, -28, -28, -28, -28, -28, -28, 0, -28, 0, -28, 0, -28, -28, -28,
        // State 26
        -48, -48, -48, -48, -48, -48, -48, -48, -48, -48, -48, -48, -48, -48, 0, -48, 0, -48, 0, -48, -48, -48,
        // State 27
        0, 21, 22, 0, 23, 24, 0, 0, 0, 0, 0, 0, 5, 25, 0, 26, 0, 27, 0, 28, 0, 0,
        // State 28
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 51, 0,
        // State 29
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 30
        -15, -15, -15, -15, -15, -15, -15, -15, -15, -15, -15, 53, -15, -15, 0, -15, 0, -15, 0, -15, -15, -15,
        // State 31
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 54, 0, 0, 0, 0, 0, 0, 0,
        // State 32
        0, 21, 22, 0, 23, 24, 0, 0, 0, 0, 0, 0, 5, 25, 0, 26, 0, 27, 0, 28, 0, 0,
        // State 33
        0, -35, -35, 0, -35, -35, 0, 0, 0, 0, 0, 0, -35, -35, 0, -35, 0, -35, 0, -35, 0, 0,
        // State 34
        0, -36, -36, 0, -36, -36, 0, 0, 0, 0, 0, 0, -36, -36, 0, -36, 0, -36, 0, -36, 0, 0,
        // State 35
        0, -38, -38, 0, -38, -38, 0, 0, 0, 0, 0, 0, -38, -38, 0, -38, 0, -38, 0, -38, 0, 0,
        // State 36
        0, -34, -34, 0, -34, -34, 0, 0, 0, 0, 0, 0, -34, -34, 0, -34, 0, -34, 0, -34, 0, 0,
        // State 37
        0, -37, -37, 0, -37, -37, 0, 0, 0, 0, 0, 0, -37, -37, 0, -37, 0, -37, 0, -37, 0, 0,
        // State 38
        0, -39, -39, 0, -39, -39, 0, 0, 0, 0, 0, 0, -39, -39, 0, -39, 0, -39, 0, -39, 0, 0,
        // State 39
        0, -25, -25, 0, -25, -25, 0, 0, 0, 0, 0, 0, -25, -25, 0, -25, 0, -25, 0, -25, -25, 0,
        // State 40
        -42, -42, -42, -42, -42, -42, -42, -42, -42, -42, -42, 56, -42, -42, 0, -42, 0, -42, 0, -42, -42, -42,
        // State 41
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 25, 59, 0, 0, 27, 0, 0, 0, 0,
        // State 42
        -50, -50, -50, -50, -50, -50, -50, -50, -50, -50, -50, -50, -50, -50, 0, -50, 0, -50, 0, -50, -50, -50,
        // State 43
        0, 0, 0, 60, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 44
        34, 0, 0, 0, 0, 0, 35, 36, 37, 38, 39, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 45
        -13, 0, 0, 61, 0, 0, -13, -13, -13, -13, -13, -13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 46
        -26, -26, -26, -26, -26, -26, -26, -26, -26, -26, -26, 0, -26, -26, 0, -26, 0, -26, 0, -26, -26, -26,
        // State 47
        -27, -27, -27, -27, -27, -27, -27, -27, -27, -27, -27, 0, -27, -27, 0, -27, 0, -27, 0, -27, -27, -27,
        // State 48
        -51, 0, 0, 0, 0, 0, -51, -51, -51, -51, -51, -51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 62,
        // State 49
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 63,
        // State 50
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 51
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -10, 0,
        // State 52
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 65, 0, 0, 0, 0, 0, 0, 0,
        // State 53
        -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, 0, -4, 0, -4, 0, -4, -4, -4,
        // State 54
        0, 0, 0, -16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -16, -16,
        // State 55
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 25, 59, 0, 0, 27, 0, 0, 0, 0,
        // State 56
        -7, -7, -7, -7, -7, -7, -7, -7, -7, -7, -7, -7, -7, -7, 0, -7, 0, -7, 0, -7, -7, -7,
        // State 57
        -33, -33, -33, -33, -33, -33, -33, -33, -33, -33, -33, -33, -33, -33, 0, -33, 0, -33, 0, -33, -33, -33,
        // State 58
        -32, -32, -32, -32, -32, -32, -32, -32, -32, -32, -32, -32, -32, -32, 0, -32, 0, -32, 0, -32, -32, -32,
        // State 59
        -41, -41, -41, -41, -41, -41, -41, -41, -41, -41, -41, -41, -41, -41, 0, -41, 0, -41, 0, -41, -41, -41,
        // State 60
        -40, -40, -40, -40, -40, -40, -40, -40, -40, -40, -40, -40, -40, -40, 0, -40, 0, -40, 0, -40, -40, -40,
        // State 61
        -17, -17, -17, -17, -17, -17, -17, -17, -17, -17, -17, -17, -17, -17, 0, -17, 0, -17, 0, -17, -17, -17,
        // State 62
        -18, -18, -18, -18, -18, -18, -18, -18, -18, -18, -18, -18, -18, -18, 0, -18, 0, -18, 0, -18, -18, -18,
        // State 63
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -11, 0,
        // State 64
        -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, 0, -5, 0, -5, 0, -5, -5, -5,
        // State 65
        -8, -8, -8, -8, -8, -8, -8, -8, -8, -8, -8, -8, -8, -8, 0, -8, 0, -8, 0, -8, -8, -8,
    ];
    const __EOF_ACTION: &'static [i8] = &[
        // State 0
        0,
        // State 1
        -19,
        // State 2
        -46,
        // State 3
        -53,
        // State 4
        -14,
        // State 5
        -51,
        // State 6
        -44,
        // State 7
        -21,
        // State 8
        -52,
        // State 9
        -24,
        // State 10
        -20,
        // State 11
        -45,
        // State 12
        -30,
        // State 13
        -13,
        // State 14
        -12,
        // State 15
        -22,
        // State 16
        -23,
        // State 17
        -29,
        // State 18
        -31,
        // State 19
        -43,
        // State 20
        0,
        // State 21
        0,
        // State 22
        0,
        // State 23
        0,
        // State 24
        -49,
        // State 25
        -28,
        // State 26
        -48,
        // State 27
        0,
        // State 28
        -47,
        // State 29
        0,
        // State 30
        -15,
        // State 31
        0,
        // State 32
        0,
        // State 33
        0,
        // State 34
        0,
        // State 35
        0,
        // State 36
        0,
        // State 37
        0,
        // State 38
        0,
        // State 39
        -25,
        // State 40
        -42,
        // State 41
        0,
        // State 42
        -50,
        // State 43
        0,
        // State 44
        0,
        // State 45
        0,
        // State 46
        -26,
        // State 47
        -27,
        // State 48
        0,
        // State 49
        0,
        // State 50
        0,
        // State 51
        -10,
        // State 52
        0,
        // State 53
        -4,
        // State 54
        -16,
        // State 55
        0,
        // State 56
        -7,
        // State 57
        -33,
        // State 58
        -32,
        // State 59
        -41,
        // State 60
        -40,
        // State 61
        -17,
        // State 62
        -18,
        // State 63
        -11,
        // State 64
        -5,
        // State 65
        -8,
    ];
    const __GOTO: &'static [i8] = &[
        // State 0
        0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0,
        // State 1
        0, 0, 0, 0, 0, 0, 0, 6, 7, 8, 9, 0, 10, 11, 12, 13, 14, 0, 0, 15, 16, 17, 0, 18, 19, 20, 0,
        // State 2
        0, 0, 0, 0, 0, 0, 29, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 3
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 4
        0, 0, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 5
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 6
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 7
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 8
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 9
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 33, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 10
        0, 0, 0, 0, 0, 0, 0, 6, 7, 0, 9, 0, 40, 0, 12, 13, 14, 0, 0, 15, 16, 17, 0, 18, 19, 20, 0,
        // State 11
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 12
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 13
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 14
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 15
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 16
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 17
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 18
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 19
        0, 0, 0, 0, 41, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 20
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 21
        0, 0, 0, 0, 0, 0, 0, 6, 7, 44, 9, 0, 45, 0, 12, 13, 46, 0, 0, 15, 16, 17, 0, 18, 19, 20, 0,
        // State 22
        0, 0, 0, 0, 0, 0, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 23
        0, 0, 0, 0, 0, 0, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 24
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 25
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 26
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 27
        0, 0, 0, 0, 0, 0, 0, 49, 7, 50, 9, 0, 45, 0, 12, 13, 14, 0, 0, 15, 16, 17, 0, 18, 19, 20, 0,
        // State 28
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 29
        0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 52, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 30
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 31
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 32
        0, 0, 0, 0, 0, 0, 0, 6, 7, 0, 9, 0, 55, 0, 12, 13, 14, 0, 0, 15, 16, 17, 0, 18, 19, 20, 0,
        // State 33
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 34
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 35
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 36
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 37
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 38
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 39
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 40
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 41
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 57, 0, 0, 0, 0, 0, 58, 0, 0, 0,
        // State 42
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 43
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 44
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 33, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 45
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 46
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 47
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 48
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 49
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 50
        0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 51
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 52
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 53
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 54
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 55
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 66, 0, 0, 0, 0, 0, 58, 0, 0, 0,
        // State 56
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 57
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 58
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 59
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 60
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 61
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 62
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 63
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 64
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 65
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    fn __expected_tokens(__state: usize) -> Vec<::std::string::String> {
        const __TERMINAL: &'static [&'static str] = &[
            r###""!=""###,
            r###""$""###,
            r###""(""###,
            r###"")""###,
            r###""-""###,
            r###""--""###,
            r###""<""###,
            r###""<=""###,
            r###""==""###,
            r###"">""###,
            r###"">=""###,
            r###""???.""###,
            r###""bare""###,
            r###""dqstring""###,
            r###""member""###,
            r###""num""###,
            r###""size""###,
            r###""sqstring""###,
            r###""variable""###,
            r###""{""###,
            r###""|""###,
            r###""}""###,
        ];
        __ACTION[(__state * 22)..].iter().zip(__TERMINAL).filter_map(|(&state, terminal)| {
            if state == 0 {
                None
            } else {
                Some(terminal.to_string())
            }
        }).collect()
    }
    pub struct __StateMachine<'input>
    where 
    {
        __phantom: ::std::marker::PhantomData<(&'input ())>,
    }
    impl<'input> __state_machine::ParserDefinition for __StateMachine<'input>
    where 
    {
        type Location = usize;
        type Error = ShellError;
        type Token = SpannedToken<'input>;
        type TokenIndex = usize;
        type Symbol = __Symbol<'input>;
        type Success = Pipeline;
        type StateIndex = i8;
        type Action = i8;
        type ReduceIndex = i8;
        type NonterminalIndex = usize;

        #[inline]
        fn start_location(&self) -> Self::Location {
              Default::default()
        }

        #[inline]
        fn start_state(&self) -> Self::StateIndex {
              0
        }

        #[inline]
        fn token_to_index(&self, token: &Self::Token) -> Option<usize> {
            __token_to_integer(token, ::std::marker::PhantomData::<(&())>)
        }

        #[inline]
        fn action(&self, state: i8, integer: usize) -> i8 {
            __ACTION[(state as usize) * 22 + integer]
        }

        #[inline]
        fn error_action(&self, state: i8) -> i8 {
            __ACTION[(state as usize) * 22 + (22 - 1)]
        }

        #[inline]
        fn eof_action(&self, state: i8) -> i8 {
            __EOF_ACTION[state as usize]
        }

        #[inline]
        fn goto(&self, state: i8, nt: usize) -> i8 {
            __GOTO[(state as usize) * 27 + nt] - 1
        }

        fn token_to_symbol(&self, token_index: usize, token: Self::Token) -> Self::Symbol {
            __token_to_symbol(token_index, token, ::std::marker::PhantomData::<(&())>)
        }

        fn expected_tokens(&self, state: i8) -> Vec<String> {
            __expected_tokens(state as usize)
        }

        #[inline]
        fn uses_error_recovery(&self) -> bool {
            false
        }

        #[inline]
        fn error_recovery_symbol(
            &self,
            recovery: __state_machine::ErrorRecovery<Self>,
        ) -> Self::Symbol {
            panic!("error recovery not enabled for this grammar")
        }

        fn reduce(
            &mut self,
            action: i8,
            start_location: Option<&Self::Location>,
            states: &mut Vec<i8>,
            symbols: &mut Vec<__state_machine::SymbolTriple<Self>>,
        ) -> Option<__state_machine::ParseResult<Self>> {
            __reduce(
                action,
                start_location,
                states,
                symbols,
                ::std::marker::PhantomData::<(&())>,
            )
        }

        fn simulate_reduce(&self, action: i8) -> __state_machine::SimulatedReduce<Self> {
            __simulate_reduce(action, ::std::marker::PhantomData::<(&())>)
        }
    }
    fn __token_to_integer<
        'input,
    >(
        __token: &SpannedToken<'input>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> Option<usize>
    {
        match *__token {
            SpannedToken { token: Token::OpNeq, .. } if true => Some(0),
            SpannedToken { token: Token::Dollar, .. } if true => Some(1),
            SpannedToken { token: Token::OpenParen, .. } if true => Some(2),
            SpannedToken { token: Token::CloseParen, .. } if true => Some(3),
            SpannedToken { token: Token::Dash, .. } if true => Some(4),
            SpannedToken { token: Token::DashDash, .. } if true => Some(5),
            SpannedToken { token: Token::OpLt, .. } if true => Some(6),
            SpannedToken { token: Token::OpLte, .. } if true => Some(7),
            SpannedToken { token: Token::OpEq, .. } if true => Some(8),
            SpannedToken { token: Token::OpGt, .. } if true => Some(9),
            SpannedToken { token: Token::OpGte, .. } if true => Some(10),
            SpannedToken { token: Token::PathDot, .. } if true => Some(11),
            SpannedToken { token: Token::Bare, .. } if true => Some(12),
            SpannedToken { token: Token::DQString, .. } if true => Some(13),
            SpannedToken { token: Token::Member, .. } if true => Some(14),
            SpannedToken { token: Token::Num, .. } if true => Some(15),
            SpannedToken { token: Token::Size, .. } if true => Some(16),
            SpannedToken { token: Token::SQString, .. } if true => Some(17),
            SpannedToken { token: Token::Variable, .. } if true => Some(18),
            SpannedToken { token: Token::OpenBrace, .. } if true => Some(19),
            SpannedToken { token: Token::Pipe, .. } if true => Some(20),
            SpannedToken { token: Token::CloseBrace, .. } if true => Some(21),
            _ => None,
        }
    }
    fn __token_to_symbol<
        'input,
    >(
        __token_index: usize,
        __token: SpannedToken<'input>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> __Symbol<'input>
    {
        match __token_index {
            0 => match __token {
                __tok @ SpannedToken { token: Token::OpNeq, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            1 => match __token {
                __tok @ SpannedToken { token: Token::Dollar, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            2 => match __token {
                __tok @ SpannedToken { token: Token::OpenParen, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            3 => match __token {
                __tok @ SpannedToken { token: Token::CloseParen, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            4 => match __token {
                __tok @ SpannedToken { token: Token::Dash, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            5 => match __token {
                __tok @ SpannedToken { token: Token::DashDash, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            6 => match __token {
                __tok @ SpannedToken { token: Token::OpLt, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            7 => match __token {
                __tok @ SpannedToken { token: Token::OpLte, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            8 => match __token {
                __tok @ SpannedToken { token: Token::OpEq, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            9 => match __token {
                __tok @ SpannedToken { token: Token::OpGt, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            10 => match __token {
                __tok @ SpannedToken { token: Token::OpGte, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            11 => match __token {
                __tok @ SpannedToken { token: Token::PathDot, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            12 => match __token {
                __tok @ SpannedToken { token: Token::Bare, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            13 => match __token {
                __tok @ SpannedToken { token: Token::DQString, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            14 => match __token {
                __tok @ SpannedToken { token: Token::Member, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            15 => match __token {
                __tok @ SpannedToken { token: Token::Num, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            16 => match __token {
                __tok @ SpannedToken { token: Token::Size, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            17 => match __token {
                __tok @ SpannedToken { token: Token::SQString, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            18 => match __token {
                __tok @ SpannedToken { token: Token::Variable, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            19 => match __token {
                __tok @ SpannedToken { token: Token::OpenBrace, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            20 => match __token {
                __tok @ SpannedToken { token: Token::Pipe, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            21 => match __token {
                __tok @ SpannedToken { token: Token::CloseBrace, .. } => __Symbol::Variant0((__tok)),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
    fn __simulate_reduce<
        'input,
    >(
        __reduce_index: i8,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> __state_machine::SimulatedReduce<__StateMachine<'input>>
    {
        match __reduce_index {
            0 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 0,
                }
            }
            1 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 0,
                    nonterminal_produced: 1,
                }
            }
            2 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 1,
                }
            }
            3 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 2,
                }
            }
            4 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 3,
                    nonterminal_produced: 2,
                }
            }
            5 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 3,
                }
            }
            6 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 4,
                }
            }
            7 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 3,
                    nonterminal_produced: 4,
                }
            }
            8 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 5,
                }
            }
            9 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 6,
                }
            }
            10 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 3,
                    nonterminal_produced: 6,
                }
            }
            11 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 7,
                }
            }
            12 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 7,
                }
            }
            13 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 8,
                }
            }
            14 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 8,
                }
            }
            15 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 3,
                    nonterminal_produced: 9,
                }
            }
            16 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 3,
                    nonterminal_produced: 10,
                }
            }
            17 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 3,
                    nonterminal_produced: 10,
                }
            }
            18 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 11,
                }
            }
            19 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 11,
                }
            }
            20 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 11,
                }
            }
            21 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 12,
                }
            }
            22 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 12,
                }
            }
            23 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 13,
                }
            }
            24 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 13,
                }
            }
            25 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 14,
                }
            }
            26 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 14,
                }
            }
            27 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 15,
                }
            }
            28 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 16,
                }
            }
            29 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 16,
                }
            }
            30 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 16,
                }
            }
            31 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 17,
                }
            }
            32 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 17,
                }
            }
            33 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 18,
                }
            }
            34 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 18,
                }
            }
            35 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 18,
                }
            }
            36 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 18,
                }
            }
            37 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 18,
                }
            }
            38 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 18,
                }
            }
            39 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 3,
                    nonterminal_produced: 19,
                }
            }
            40 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 3,
                    nonterminal_produced: 19,
                }
            }
            41 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 20,
                }
            }
            42 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 21,
                }
            }
            43 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 21,
                }
            }
            44 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 21,
                }
            }
            45 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 22,
                }
            }
            46 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 22,
                }
            }
            47 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 23,
                }
            }
            48 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 23,
                }
            }
            49 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 24,
                }
            }
            50 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 25,
                }
            }
            51 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 25,
                }
            }
            52 => __state_machine::SimulatedReduce::Accept,
            _ => panic!("invalid reduction index {}", __reduce_index)
        }
    }
    pub struct PipelineParser {
        _priv: (),
    }

    impl PipelineParser {
        pub fn new() -> PipelineParser {
            PipelineParser {
                _priv: (),
            }
        }

        #[allow(dead_code)]
        pub fn parse<
            'input,
            __TOKEN: __ToTriple<'input, >,
            __TOKENS: IntoIterator<Item=__TOKEN>,
        >(
            &self,
            __tokens0: __TOKENS,
        ) -> Result<Pipeline, __lalrpop_util::ParseError<usize, SpannedToken<'input>, ShellError>>
        {
            let __tokens = __tokens0.into_iter();
            let mut __tokens = __tokens.map(|t| __ToTriple::to_triple(t));
            let __r = __state_machine::Parser::drive(
                __StateMachine {
                    __phantom: ::std::marker::PhantomData::<(&())>,
                },
                __tokens,
            );
            __r
        }
    }
    pub(crate) fn __reduce<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> Option<Result<Pipeline,__lalrpop_util::ParseError<usize, SpannedToken<'input>, ShellError>>>
    {
        let (__pop_states, __nonterminal) = match __action {
            0 => {
                __reduce0(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            1 => {
                __reduce1(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            2 => {
                __reduce2(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            3 => {
                __reduce3(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            4 => {
                __reduce4(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            5 => {
                __reduce5(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            6 => {
                __reduce6(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            7 => {
                __reduce7(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            8 => {
                __reduce8(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            9 => {
                __reduce9(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            10 => {
                __reduce10(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            11 => {
                __reduce11(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            12 => {
                __reduce12(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            13 => {
                __reduce13(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            14 => {
                __reduce14(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            15 => {
                __reduce15(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            16 => {
                __reduce16(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            17 => {
                __reduce17(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            18 => {
                __reduce18(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            19 => {
                __reduce19(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            20 => {
                __reduce20(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            21 => {
                __reduce21(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            22 => {
                __reduce22(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            23 => {
                __reduce23(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            24 => {
                __reduce24(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            25 => {
                __reduce25(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            26 => {
                __reduce26(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            27 => {
                __reduce27(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            28 => {
                __reduce28(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            29 => {
                __reduce29(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            30 => {
                __reduce30(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            31 => {
                __reduce31(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            32 => {
                __reduce32(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            33 => {
                __reduce33(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            34 => {
                __reduce34(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            35 => {
                __reduce35(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            36 => {
                __reduce36(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            37 => {
                __reduce37(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            38 => {
                __reduce38(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            39 => {
                __reduce39(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            40 => {
                __reduce40(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            41 => {
                __reduce41(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            42 => {
                __reduce42(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            43 => {
                __reduce43(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            44 => {
                __reduce44(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            45 => {
                __reduce45(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            46 => {
                __reduce46(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            47 => {
                __reduce47(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            48 => {
                __reduce48(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            49 => {
                __reduce49(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            50 => {
                __reduce50(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            51 => {
                __reduce51(__action, __lookahead_start, __states, __symbols, ::std::marker::PhantomData::<(&())>)
            }
            52 => {
                // __Pipeline = Pipeline => ActionFn(0);
                let __sym0 = __pop_Variant12(__symbols);
                let __start = __sym0.0.clone();
                let __end = __sym0.2.clone();
                let __nt = super::__action0::<>(__sym0);
                return Some(Ok(__nt));
            }
            _ => panic!("invalid action code {}", __action)
        };
        let __states_len = __states.len();
        __states.truncate(__states_len - __pop_states);
        let __state = *__states.last().unwrap() as usize;
        let __next_state = __GOTO[__state * 27 + __nonterminal] - 1;
        __states.push(__next_state);
        None
    }
    fn __pop_Variant7<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, BarePath, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant7(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant6<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, Expression, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant6(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant9<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, Flag, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant9(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant11<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, Operator, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant11(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant4<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, ParsedCommand, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant4(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant12<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, Pipeline, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant12(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant0<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, SpannedToken<'input>, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant0(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant2<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, String, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant2(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant13<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, Variable, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant13(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant10<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, i64, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant10(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant8<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, ::std::vec::Vec<Expression>, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant8(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant5<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, ::std::vec::Vec<ParsedCommand>, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant5(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant1<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, ::std::vec::Vec<SpannedToken<'input>>, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant1(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    fn __pop_Variant3<
      'input,
    >(
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>
    ) -> (usize, ::std::vec::Vec<String>, usize)
     {
        match __symbols.pop().unwrap() {
            (__l, __Symbol::Variant3(__v), __r) => (__l, __v, __r),
            _ => panic!("symbol type mismatch")
        }
    }
    pub(crate) fn __reduce0<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("???." <"member">) = "???.", "member" => ActionFn(41);
        let __sym1 = __pop_Variant0(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action41::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant0(__nt), __end));
        (2, 0)
    }
    pub(crate) fn __reduce1<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("???." <"member">)* =  => ActionFn(39);
        let __start = __symbols.last().map(|s| s.2.clone()).unwrap_or_default();
        let __end = __lookahead_start.cloned().unwrap_or_else(|| __start.clone());
        let __nt = super::__action39::<>(&__start, &__end);
        __symbols.push((__start, __Symbol::Variant1(__nt), __end));
        (0, 1)
    }
    pub(crate) fn __reduce2<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("???." <"member">)* = ("???." <"member">)+ => ActionFn(40);
        let __sym0 = __pop_Variant1(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action40::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant1(__nt), __end));
        (1, 1)
    }
    pub(crate) fn __reduce3<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("???." <"member">)+ = "???.", "member" => ActionFn(52);
        let __sym1 = __pop_Variant0(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action52::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant1(__nt), __end));
        (2, 2)
    }
    pub(crate) fn __reduce4<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("???." <"member">)+ = ("???." <"member">)+, "???.", "member" => ActionFn(53);
        let __sym2 = __pop_Variant0(__symbols);
        let __sym1 = __pop_Variant0(__symbols);
        let __sym0 = __pop_Variant1(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym2.2.clone();
        let __nt = super::__action53::<>(__sym0, __sym1, __sym2);
        __symbols.push((__start, __Symbol::Variant1(__nt), __end));
        (3, 2)
    }
    pub(crate) fn __reduce5<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("???." <Member>) = "???.", Member => ActionFn(44);
        let __sym1 = __pop_Variant2(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action44::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant2(__nt), __end));
        (2, 3)
    }
    pub(crate) fn __reduce6<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("???." <Member>)+ = "???.", Member => ActionFn(56);
        let __sym1 = __pop_Variant2(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action56::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant3(__nt), __end));
        (2, 4)
    }
    pub(crate) fn __reduce7<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("???." <Member>)+ = ("???." <Member>)+, "???.", Member => ActionFn(57);
        let __sym2 = __pop_Variant2(__symbols);
        let __sym1 = __pop_Variant0(__symbols);
        let __sym0 = __pop_Variant3(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym2.2.clone();
        let __nt = super::__action57::<>(__sym0, __sym1, __sym2);
        __symbols.push((__start, __Symbol::Variant3(__nt), __end));
        (3, 4)
    }
    pub(crate) fn __reduce8<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("|" <Command>) = "|", Command => ActionFn(49);
        let __sym1 = __pop_Variant4(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action49::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant4(__nt), __end));
        (2, 5)
    }
    pub(crate) fn __reduce9<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("|" <Command>)+ = "|", Command => ActionFn(58);
        let __sym1 = __pop_Variant4(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action58::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant5(__nt), __end));
        (2, 6)
    }
    pub(crate) fn __reduce10<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // ("|" <Command>)+ = ("|" <Command>)+, "|", Command => ActionFn(59);
        let __sym2 = __pop_Variant4(__symbols);
        let __sym1 = __pop_Variant0(__symbols);
        let __sym0 = __pop_Variant5(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym2.2.clone();
        let __nt = super::__action59::<>(__sym0, __sym1, __sym2);
        __symbols.push((__start, __Symbol::Variant5(__nt), __end));
        (3, 6)
    }
    pub(crate) fn __reduce11<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // AtomicExpression = Parenthesized => ActionFn(12);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action12::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 7)
    }
    pub(crate) fn __reduce12<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // AtomicExpression = Leaf => ActionFn(13);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action13::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 7)
    }
    pub(crate) fn __reduce13<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // BarePath = "bare" => ActionFn(54);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action54::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant7(__nt), __end));
        (1, 8)
    }
    pub(crate) fn __reduce14<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // BarePath = "bare", ("???." <"member">)+ => ActionFn(55);
        let __sym1 = __pop_Variant1(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action55::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant7(__nt), __end));
        (2, 8)
    }
    pub(crate) fn __reduce15<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // BinaryExpression = Expr, Operator, Expr => ActionFn(9);
        let __sym2 = __pop_Variant6(__symbols);
        let __sym1 = __pop_Variant11(__symbols);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym2.2.clone();
        let __nt = super::__action9::<>(__sym0, __sym1, __sym2);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (3, 9)
    }
    pub(crate) fn __reduce16<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Block = "{", AtomicExpression, "}" => ActionFn(14);
        let __sym2 = __pop_Variant0(__symbols);
        let __sym1 = __pop_Variant6(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym2.2.clone();
        let __nt = super::__action14::<>(__sym0, __sym1, __sym2);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (3, 10)
    }
    pub(crate) fn __reduce17<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Block = "{", BinaryExpression, "}" => ActionFn(15);
        let __sym2 = __pop_Variant0(__symbols);
        let __sym1 = __pop_Variant6(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym2.2.clone();
        let __nt = super::__action15::<>(__sym0, __sym1, __sym2);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (3, 10)
    }
    pub(crate) fn __reduce18<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Command = BarePath => ActionFn(3);
        let __sym0 = __pop_Variant7(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action3::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant4(__nt), __end));
        (1, 11)
    }
    pub(crate) fn __reduce19<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Command = BarePath, Expr+ => ActionFn(4);
        let __sym1 = __pop_Variant8(__symbols);
        let __sym0 = __pop_Variant7(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action4::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant4(__nt), __end));
        (2, 11)
    }
    pub(crate) fn __reduce20<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Command = BarePath, BinaryExpression => ActionFn(5);
        let __sym1 = __pop_Variant6(__symbols);
        let __sym0 = __pop_Variant7(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action5::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant4(__nt), __end));
        (2, 11)
    }
    pub(crate) fn __reduce21<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Expr = PathExpression => ActionFn(22);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action22::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 12)
    }
    pub(crate) fn __reduce22<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Expr = PathHead => ActionFn(23);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action23::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 12)
    }
    pub(crate) fn __reduce23<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Expr+ = Expr => ActionFn(45);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action45::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant8(__nt), __end));
        (1, 13)
    }
    pub(crate) fn __reduce24<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Expr+ = Expr+, Expr => ActionFn(46);
        let __sym1 = __pop_Variant6(__symbols);
        let __sym0 = __pop_Variant8(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action46::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant8(__nt), __end));
        (2, 13)
    }
    pub(crate) fn __reduce25<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Flag = "-", BarePath => ActionFn(33);
        let __sym1 = __pop_Variant7(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action33::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant9(__nt), __end));
        (2, 14)
    }
    pub(crate) fn __reduce26<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Flag = "--", BarePath => ActionFn(34);
        let __sym1 = __pop_Variant7(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action34::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant9(__nt), __end));
        (2, 14)
    }
    pub(crate) fn __reduce27<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Int = "num" => ActionFn(38);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action38::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant10(__nt), __end));
        (1, 15)
    }
    pub(crate) fn __reduce28<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Leaf = String => ActionFn(6);
        let __sym0 = __pop_Variant2(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action6::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 16)
    }
    pub(crate) fn __reduce29<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Leaf = Int => ActionFn(7);
        let __sym0 = __pop_Variant10(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action7::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 16)
    }
    pub(crate) fn __reduce30<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Leaf = Variable => ActionFn(8);
        let __sym0 = __pop_Variant13(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action8::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 16)
    }
    pub(crate) fn __reduce31<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Member = "member" => ActionFn(25);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action25::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant2(__nt), __end));
        (1, 17)
    }
    pub(crate) fn __reduce32<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Member = String => ActionFn(26);
        let __sym0 = __pop_Variant2(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action26::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant2(__nt), __end));
        (1, 17)
    }
    pub(crate) fn __reduce33<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Operator = "==" => ActionFn(27);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action27::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant11(__nt), __end));
        (1, 18)
    }
    pub(crate) fn __reduce34<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Operator = "!=" => ActionFn(28);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action28::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant11(__nt), __end));
        (1, 18)
    }
    pub(crate) fn __reduce35<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Operator = "<" => ActionFn(29);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action29::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant11(__nt), __end));
        (1, 18)
    }
    pub(crate) fn __reduce36<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Operator = ">" => ActionFn(30);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action30::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant11(__nt), __end));
        (1, 18)
    }
    pub(crate) fn __reduce37<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Operator = "<=" => ActionFn(31);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action31::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant11(__nt), __end));
        (1, 18)
    }
    pub(crate) fn __reduce38<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Operator = ">=" => ActionFn(32);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action32::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant11(__nt), __end));
        (1, 18)
    }
    pub(crate) fn __reduce39<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Parenthesized = "(", Leaf, ")" => ActionFn(10);
        let __sym2 = __pop_Variant0(__symbols);
        let __sym1 = __pop_Variant6(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym2.2.clone();
        let __nt = super::__action10::<>(__sym0, __sym1, __sym2);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (3, 19)
    }
    pub(crate) fn __reduce40<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Parenthesized = "(", BinaryExpression, ")" => ActionFn(11);
        let __sym2 = __pop_Variant0(__symbols);
        let __sym1 = __pop_Variant6(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym2.2.clone();
        let __nt = super::__action11::<>(__sym0, __sym1, __sym2);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (3, 19)
    }
    pub(crate) fn __reduce41<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // PathExpression = WholeExpression, ("???." <Member>)+ => ActionFn(21);
        let __sym1 = __pop_Variant3(__symbols);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action21::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (2, 20)
    }
    pub(crate) fn __reduce42<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // PathHead = WholeExpression => ActionFn(18);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action18::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 21)
    }
    pub(crate) fn __reduce43<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // PathHead = BarePath => ActionFn(19);
        let __sym0 = __pop_Variant7(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action19::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 21)
    }
    pub(crate) fn __reduce44<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // PathHead = Flag => ActionFn(20);
        let __sym0 = __pop_Variant9(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action20::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 21)
    }
    pub(crate) fn __reduce45<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Pipeline = Command => ActionFn(1);
        let __sym0 = __pop_Variant4(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action1::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant12(__nt), __end));
        (1, 22)
    }
    pub(crate) fn __reduce46<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Pipeline = Command, ("|" <Command>)+ => ActionFn(2);
        let __sym1 = __pop_Variant5(__symbols);
        let __sym0 = __pop_Variant4(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action2::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant12(__nt), __end));
        (2, 22)
    }
    pub(crate) fn __reduce47<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // String = "sqstring" => ActionFn(35);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action35::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant2(__nt), __end));
        (1, 23)
    }
    pub(crate) fn __reduce48<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // String = "dqstring" => ActionFn(36);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action36::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant2(__nt), __end));
        (1, 23)
    }
    pub(crate) fn __reduce49<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // Variable = "$", "variable" => ActionFn(24);
        let __sym1 = __pop_Variant0(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym1.2.clone();
        let __nt = super::__action24::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant13(__nt), __end));
        (2, 24)
    }
    pub(crate) fn __reduce50<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // WholeExpression = AtomicExpression => ActionFn(16);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action16::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 25)
    }
    pub(crate) fn __reduce51<
        'input,
    >(
        __action: i8,
        __lookahead_start: Option<&usize>,
        __states: &mut ::std::vec::Vec<i8>,
        __symbols: &mut ::std::vec::Vec<(usize,__Symbol<'input>,usize)>,
        _: ::std::marker::PhantomData<(&'input ())>,
    ) -> (usize, usize)
    {
        // WholeExpression = Block => ActionFn(17);
        let __sym0 = __pop_Variant6(__symbols);
        let __start = __sym0.0.clone();
        let __end = __sym0.2.clone();
        let __nt = super::__action17::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant6(__nt), __end));
        (1, 25)
    }
}
pub use self::__parse__Pipeline::PipelineParser;

fn __action0<
    'input,
>(
    (_, __0, _): (usize, Pipeline, usize),
) -> Pipeline
{
    (__0)
}

fn __action1<
    'input,
>(
    (_, first, _): (usize, ParsedCommand, usize),
) -> Pipeline
{
    Pipeline::new(vec![first])
}

fn __action2<
    'input,
>(
    (_, first, _): (usize, ParsedCommand, usize),
    (_, rest, _): (usize, ::std::vec::Vec<ParsedCommand>, usize),
) -> Pipeline
{
    Pipeline::from_parts(first, rest)
}

fn __action3<
    'input,
>(
    (_, command, _): (usize, BarePath, usize),
) -> ParsedCommand
{
    ParsedCommand::new(command.to_string(), vec![])
}

fn __action4<
    'input,
>(
    (_, command, _): (usize, BarePath, usize),
    (_, expr, _): (usize, ::std::vec::Vec<Expression>, usize),
) -> ParsedCommand
{
    ParsedCommand::new(command.to_string(), expr)
}

fn __action5<
    'input,
>(
    (_, command, _): (usize, BarePath, usize),
    (_, expr, _): (usize, Expression, usize),
) -> ParsedCommand
{
    ParsedCommand::new(command.to_string(), vec![expr])
}

fn __action6<
    'input,
>(
    (_, __0, _): (usize, String, usize),
) -> Expression
{
    Expression::Leaf(Leaf::String(__0))
}

fn __action7<
    'input,
>(
    (_, __0, _): (usize, i64, usize),
) -> Expression
{
    Expression::Leaf(Leaf::Int(__0))
}

fn __action8<
    'input,
>(
    (_, __0, _): (usize, Variable, usize),
) -> Expression
{
    Expression::VariableReference(__0)
}

fn __action9<
    'input,
>(
    (_, left, _): (usize, Expression, usize),
    (_, op, _): (usize, Operator, usize),
    (_, right, _): (usize, Expression, usize),
) -> Expression
{
    Expression::Binary(Box::new(Binary::new(left, op, right)))
}

fn __action10<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, Expression, usize),
    (_, _, _): (usize, SpannedToken<'input>, usize),
) -> Expression
{
    Expression::Parenthesized(Box::new(Parenthesized::new(__0)))
}

fn __action11<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, Expression, usize),
    (_, _, _): (usize, SpannedToken<'input>, usize),
) -> Expression
{
    Expression::Parenthesized(Box::new(Parenthesized::new(__0)))
}

fn __action12<
    'input,
>(
    (_, __0, _): (usize, Expression, usize),
) -> Expression
{
    (__0)
}

fn __action13<
    'input,
>(
    (_, __0, _): (usize, Expression, usize),
) -> Expression
{
    (__0)
}

fn __action14<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, Expression, usize),
    (_, _, _): (usize, SpannedToken<'input>, usize),
) -> Expression
{
    Expression::Block(Box::new(Block::new(__0)))
}

fn __action15<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, Expression, usize),
    (_, _, _): (usize, SpannedToken<'input>, usize),
) -> Expression
{
    Expression::Block(Box::new(Block::new(__0)))
}

fn __action16<
    'input,
>(
    (_, __0, _): (usize, Expression, usize),
) -> Expression
{
    (__0)
}

fn __action17<
    'input,
>(
    (_, __0, _): (usize, Expression, usize),
) -> Expression
{
    (__0)
}

fn __action18<
    'input,
>(
    (_, __0, _): (usize, Expression, usize),
) -> Expression
{
    (__0)
}

fn __action19<
    'input,
>(
    (_, __0, _): (usize, BarePath, usize),
) -> Expression
{
    Expression::Leaf(Leaf::Bare(__0))
}

fn __action20<
    'input,
>(
    (_, __0, _): (usize, Flag, usize),
) -> Expression
{
    Expression::Flag(__0)
}

fn __action21<
    'input,
>(
    (_, head, _): (usize, Expression, usize),
    (_, tail, _): (usize, ::std::vec::Vec<String>, usize),
) -> Expression
{
    Expression::Path(Box::new(Path::new(head, tail)))
}

fn __action22<
    'input,
>(
    (_, __0, _): (usize, Expression, usize),
) -> Expression
{
    (__0)
}

fn __action23<
    'input,
>(
    (_, __0, _): (usize, Expression, usize),
) -> Expression
{
    (__0)
}

fn __action24<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> Variable
{
    Variable::from_str(__0.as_slice()).unwrap()
}

fn __action25<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> String
{
    __0.to_string()
}

fn __action26<
    'input,
>(
    (_, __0, _): (usize, String, usize),
) -> String
{
    (__0)
}

fn __action27<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> Operator
{
    Operator::Equal
}

fn __action28<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> Operator
{
    Operator::NotEqual
}

fn __action29<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> Operator
{
    Operator::LessThan
}

fn __action30<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> Operator
{
    Operator::GreaterThan
}

fn __action31<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> Operator
{
    Operator::LessThanOrEqual
}

fn __action32<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> Operator
{
    Operator::GreaterThanOrEqual
}

fn __action33<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, BarePath, usize),
) -> Flag
{
    Flag::Shorthand(__0.to_string())
}

fn __action34<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, BarePath, usize),
) -> Flag
{
    Flag::Longhand(__0.to_string())
}

fn __action35<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> String
{
    __0.as_slice()[1..(__0.as_slice().len() - 1)].to_string()
}

fn __action36<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> String
{
    __0.as_slice()[1..(__0.as_slice().len() - 1)].to_string()
}

fn __action37<
    'input,
>(
    (_, head, _): (usize, SpannedToken<'input>, usize),
    (_, tail, _): (usize, ::std::vec::Vec<SpannedToken<'input>>, usize),
) -> BarePath
{
    BarePath::from_tokens(head, tail)
}

fn __action38<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> i64
{
    i64::from_str(__0.as_slice()).unwrap()
}

fn __action39<
    'input,
>(
    __lookbehind: &usize,
    __lookahead: &usize,
) -> ::std::vec::Vec<SpannedToken<'input>>
{
    vec![]
}

fn __action40<
    'input,
>(
    (_, v, _): (usize, ::std::vec::Vec<SpannedToken<'input>>, usize),
) -> ::std::vec::Vec<SpannedToken<'input>>
{
    v
}

fn __action41<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> SpannedToken<'input>
{
    (__0)
}

fn __action42<
    'input,
>(
    (_, __0, _): (usize, String, usize),
) -> ::std::vec::Vec<String>
{
    vec![__0]
}

fn __action43<
    'input,
>(
    (_, v, _): (usize, ::std::vec::Vec<String>, usize),
    (_, e, _): (usize, String, usize),
) -> ::std::vec::Vec<String>
{
    { let mut v = v; v.push(e); v }
}

fn __action44<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, String, usize),
) -> String
{
    (__0)
}

fn __action45<
    'input,
>(
    (_, __0, _): (usize, Expression, usize),
) -> ::std::vec::Vec<Expression>
{
    vec![__0]
}

fn __action46<
    'input,
>(
    (_, v, _): (usize, ::std::vec::Vec<Expression>, usize),
    (_, e, _): (usize, Expression, usize),
) -> ::std::vec::Vec<Expression>
{
    { let mut v = v; v.push(e); v }
}

fn __action47<
    'input,
>(
    (_, __0, _): (usize, ParsedCommand, usize),
) -> ::std::vec::Vec<ParsedCommand>
{
    vec![__0]
}

fn __action48<
    'input,
>(
    (_, v, _): (usize, ::std::vec::Vec<ParsedCommand>, usize),
    (_, e, _): (usize, ParsedCommand, usize),
) -> ::std::vec::Vec<ParsedCommand>
{
    { let mut v = v; v.push(e); v }
}

fn __action49<
    'input,
>(
    (_, _, _): (usize, SpannedToken<'input>, usize),
    (_, __0, _): (usize, ParsedCommand, usize),
) -> ParsedCommand
{
    (__0)
}

fn __action50<
    'input,
>(
    (_, __0, _): (usize, SpannedToken<'input>, usize),
) -> ::std::vec::Vec<SpannedToken<'input>>
{
    vec![__0]
}

fn __action51<
    'input,
>(
    (_, v, _): (usize, ::std::vec::Vec<SpannedToken<'input>>, usize),
    (_, e, _): (usize, SpannedToken<'input>, usize),
) -> ::std::vec::Vec<SpannedToken<'input>>
{
    { let mut v = v; v.push(e); v }
}

fn __action52<
    'input,
>(
    __0: (usize, SpannedToken<'input>, usize),
    __1: (usize, SpannedToken<'input>, usize),
) -> ::std::vec::Vec<SpannedToken<'input>>
{
    let __start0 = __0.0.clone();
    let __end0 = __1.2.clone();
    let __temp0 = __action41(
        __0,
        __1,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action50(
        __temp0,
    )
}

fn __action53<
    'input,
>(
    __0: (usize, ::std::vec::Vec<SpannedToken<'input>>, usize),
    __1: (usize, SpannedToken<'input>, usize),
    __2: (usize, SpannedToken<'input>, usize),
) -> ::std::vec::Vec<SpannedToken<'input>>
{
    let __start0 = __1.0.clone();
    let __end0 = __2.2.clone();
    let __temp0 = __action41(
        __1,
        __2,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action51(
        __0,
        __temp0,
    )
}

fn __action54<
    'input,
>(
    __0: (usize, SpannedToken<'input>, usize),
) -> BarePath
{
    let __start0 = __0.2.clone();
    let __end0 = __0.2.clone();
    let __temp0 = __action39(
        &__start0,
        &__end0,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action37(
        __0,
        __temp0,
    )
}

fn __action55<
    'input,
>(
    __0: (usize, SpannedToken<'input>, usize),
    __1: (usize, ::std::vec::Vec<SpannedToken<'input>>, usize),
) -> BarePath
{
    let __start0 = __1.0.clone();
    let __end0 = __1.2.clone();
    let __temp0 = __action40(
        __1,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action37(
        __0,
        __temp0,
    )
}

fn __action56<
    'input,
>(
    __0: (usize, SpannedToken<'input>, usize),
    __1: (usize, String, usize),
) -> ::std::vec::Vec<String>
{
    let __start0 = __0.0.clone();
    let __end0 = __1.2.clone();
    let __temp0 = __action44(
        __0,
        __1,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action42(
        __temp0,
    )
}

fn __action57<
    'input,
>(
    __0: (usize, ::std::vec::Vec<String>, usize),
    __1: (usize, SpannedToken<'input>, usize),
    __2: (usize, String, usize),
) -> ::std::vec::Vec<String>
{
    let __start0 = __1.0.clone();
    let __end0 = __2.2.clone();
    let __temp0 = __action44(
        __1,
        __2,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action43(
        __0,
        __temp0,
    )
}

fn __action58<
    'input,
>(
    __0: (usize, SpannedToken<'input>, usize),
    __1: (usize, ParsedCommand, usize),
) -> ::std::vec::Vec<ParsedCommand>
{
    let __start0 = __0.0.clone();
    let __end0 = __1.2.clone();
    let __temp0 = __action49(
        __0,
        __1,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action47(
        __temp0,
    )
}

fn __action59<
    'input,
>(
    __0: (usize, ::std::vec::Vec<ParsedCommand>, usize),
    __1: (usize, SpannedToken<'input>, usize),
    __2: (usize, ParsedCommand, usize),
) -> ::std::vec::Vec<ParsedCommand>
{
    let __start0 = __1.0.clone();
    let __end0 = __2.2.clone();
    let __temp0 = __action49(
        __1,
        __2,
    );
    let __temp0 = (__start0, __temp0, __end0);
    __action48(
        __0,
        __temp0,
    )
}

pub trait __ToTriple<'input, > {
    fn to_triple(value: Self) -> Result<(usize,SpannedToken<'input>,usize), __lalrpop_util::ParseError<usize, SpannedToken<'input>, ShellError>>;
}

impl<'input, > __ToTriple<'input, > for (usize, SpannedToken<'input>, usize) {
    fn to_triple(value: Self) -> Result<(usize,SpannedToken<'input>,usize), __lalrpop_util::ParseError<usize, SpannedToken<'input>, ShellError>> {
        Ok(value)
    }
}
impl<'input, > __ToTriple<'input, > for Result<(usize, SpannedToken<'input>, usize), ShellError> {
    fn to_triple(value: Self) -> Result<(usize,SpannedToken<'input>,usize), __lalrpop_util::ParseError<usize, SpannedToken<'input>, ShellError>> {
        match value {
            Ok(v) => Ok(v),
            Err(error) => Err(__lalrpop_util::ParseError::User { error }),
        }
    }
}
