mod bits;
mod conversions;
mod filters;
mod formats;
mod math;
mod platform;
mod strings;

pub use bits::{
    Bits, BitsAnd, BitsInto, BitsNot, BitsOr, BitsRol, BitsRor, BitsShl, BitsShr, BitsXor,
};
pub use formats::ToHtml;
pub use math::{MathArcCos, MathArcCosH, MathArcSin, MathArcSinH, MathArcTan, MathArcTanH};
pub use math::{MathCos, MathCosH, MathSin, MathSinH, MathTan, MathTanH};
pub use math::{MathExp, MathLn};

use nu_protocol::engine::{EngineState, StateWorkingSet};

pub fn add_extra_command_context(mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        bind_command!(conversions::Fmt);

        bind_command!(
            filters::UpdateCells,
            filters::EachWhile,
            filters::Roll,
            filters::RollDown,
            filters::RollUp,
            filters::RollLeft,
            filters::RollRight,
            filters::Rotate
        );

        bind_command!(platform::ansi::Gradient);

        bind_command!(
            strings::format::FormatPattern,
            strings::str_::case::Str,
            strings::str_::case::StrCamelCase,
            strings::str_::case::StrKebabCase,
            strings::str_::case::StrPascalCase,
            strings::str_::case::StrScreamingSnakeCase,
            strings::str_::case::StrSnakeCase,
            strings::str_::case::StrTitleCase
        );

        bind_command!(ToHtml, formats::FromUrl);

        // Bits
        bind_command! {
            Bits,
            BitsAnd,
            BitsInto,
            BitsNot,
            BitsOr,
            BitsRol,
            BitsRor,
            BitsShl,
            BitsShr,
            BitsXor
        }

        // Math
        bind_command! {
            MathArcSin,
            MathArcCos,
            MathArcTan,
            MathArcSinH,
            MathArcCosH,
            MathArcTanH,
            MathSin,
            MathCos,
            MathTan,
            MathSinH,
            MathCosH,
            MathTanH,
            MathExp,
            MathLn
        };

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating extra command context: {err:?}");
    }

    engine_state
}
