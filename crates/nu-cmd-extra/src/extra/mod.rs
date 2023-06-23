mod bits;
mod bytes;

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
