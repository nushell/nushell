#![no_main]

use libfuzzer_sys::fuzz_target;

use nu_parser::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

fuzz_target!(|data: &[u8]| {
    let engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let _block = parse(&mut working_set, None, &data, true);
});
