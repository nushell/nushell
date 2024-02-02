mod bits;
mod conversions;
mod filters;
mod formats;
mod math;
mod platform;
mod strings;

pub use bits::Bits;
pub use bits::BitsAnd;
pub use bits::BitsInto;
pub use bits::BitsNot;
pub use bits::BitsOr;
pub use bits::BitsRol;
pub use bits::BitsRor;
pub use bits::BitsShl;
pub use bits::BitsShr;
pub use bits::BitsXor;

pub use math::MathCos;
pub use math::MathCosH;
pub use math::MathSin;
pub use math::MathSinH;
pub use math::MathTan;
pub use math::MathTanH;

pub use math::MathExp;
pub use math::MathLn;

pub use math::MathArcCos;
pub use math::MathArcCosH;
pub use math::MathArcSin;
pub use math::MathArcSinH;
pub use math::MathArcTan;
pub use math::MathArcTanH;

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
            strings::encode_decode::EncodeHex,
            strings::encode_decode::DecodeHex,
            strings::str_::case::Str,
            strings::str_::case::StrCamelCase,
            strings::str_::case::StrKebabCase,
            strings::str_::case::StrPascalCase,
            strings::str_::case::StrScreamingSnakeCase,
            strings::str_::case::StrSnakeCase,
            strings::str_::case::StrTitleCase
        );

        bind_command!(formats::ToHtml, formats::FromUrl);
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
