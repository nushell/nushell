mod bits;

pub use bits::add_bits_decls;

use nu_protocol::engine::StateWorkingSet;

pub fn add_extra_decls(working_set: &mut StateWorkingSet) {
    add_bits_decls(working_set);
}
