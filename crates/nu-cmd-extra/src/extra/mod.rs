mod bits;
mod conversions;
mod filters;
mod formats;
mod platform;
mod strings;

use nu_protocol::engine::StateWorkingSet;

pub fn add_extra_decls(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    bind_command!(
        bits::bits_::Bits,
        bits::and::BitsAnd,
        bits::not::BitsNot,
        bits::or::BitsOr,
        bits::xor::BitsXor,
        bits::rotate_left::BitsRol,
        bits::rotate_right::BitsRor,
        bits::shift_left::BitsShl,
        bits::shift_right::BitsShr
    );

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
        strings::format::Format,
        strings::format::FileSize,
        strings::encode_decode::EncodeHex,
        strings::encode_decode::DecodeHex
    );

    bind_command!(formats::ToHtml, formats::FromUrl);
}
