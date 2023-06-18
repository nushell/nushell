mod bits;

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
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating extra command context: {err:?}");
    }

    engine_state
}
