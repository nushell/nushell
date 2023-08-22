mod bits;
mod bytes;
mod conversions;
mod filters;
mod formats;
mod platform;
mod strings;

pub use bytes::Bytes;
pub use bytes::BytesAdd;
pub use bytes::BytesAt;
pub use bytes::BytesBuild;
pub use bytes::BytesCollect;
pub use bytes::BytesEndsWith;
pub use bytes::BytesIndexOf;
pub use bytes::BytesLen;
pub use bytes::BytesRemove;
pub use bytes::BytesReplace;
pub use bytes::BytesReverse;
pub use bytes::BytesStartsWith;

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

        bind_command!(platform::ansi::Gradient, platform::ansi::Link);

        bind_command!(
            strings::format::Format,
            strings::format::FileSize,
            strings::encode_decode::EncodeHex,
            strings::encode_decode::DecodeHex
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

        // Bytes
        bind_command! {
            Bytes,
            BytesLen,
            BytesStartsWith,
            BytesEndsWith,
            BytesReverse,
            BytesReplace,
            BytesAdd,
            BytesAt,
            BytesIndexOf,
            BytesCollect,
            BytesRemove,
            BytesBuild
        }

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating extra command context: {err:?}");
    }

    engine_state
}
