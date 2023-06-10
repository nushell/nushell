mod bits;

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
}
