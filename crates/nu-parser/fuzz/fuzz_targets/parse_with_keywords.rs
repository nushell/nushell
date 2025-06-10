#![no_main]

use libfuzzer_sys::fuzz_target;

use nu_cmd_lang::create_default_context;
use nu_parser::*;
use nu_protocol::engine::StateWorkingSet;

fuzz_target!(|data: &[u8]| {
    let engine_state = create_default_context();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let _block = parse(&mut working_set, None, &data, true);
});
