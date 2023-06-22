mod bits;
mod bytes;

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

        bind_command! {
            bits::bits_::Bits,
            bits::and::BitsAnd,
            bits::not::BitsNot,
            bits::or::BitsOr,
            bits::xor::BitsXor,
            bits::rotate_left::BitsRol,
            bits::rotate_right::BitsRor,
            bits::shift_left::BitsShl,
            bits::shift_right::BitsShr
        }

        // Bytes
        bind_command! {
            bytes::bytes_::Bytes,
            bytes::length::BytesLen,
            bytes::starts_with::BytesStartsWith,
            bytes::ends_with::BytesEndsWith,
            bytes::reverse::BytesReverse,
            bytes::replace::BytesReplace,
            bytes::add::BytesAdd,
            bytes::at::BytesAt,
            bytes::index_of::BytesIndexOf,
            bytes::collect::BytesCollect,
            bytes::remove::BytesRemove,
            bytes::build_::BytesBuild
        }

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating extra command context: {err:?}");
    }

    engine_state
}
