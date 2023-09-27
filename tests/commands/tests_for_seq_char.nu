use std assert

# Parameter name:
# sig type   : nothing
# name       : start
# type       : positional
# shape      : string
# description: start of character sequence (inclusive)

# Parameter name:
# sig type   : nothing
# name       : end
# type       : positional
# shape      : string
# description: end of character sequence (inclusive)


# This is the custom command 1 for seq_char:

#[test]
def seq_char_sequence_a_to_e_1 [] {
  let result = (seq char a e)
  assert ($result == [a, b, c, d, e])
}

# This is the custom command 2 for seq_char:

#[test]
def seq_char_sequence_a_to_e_and_put_the_characters_in_a_pipe_separated_string_2 [] {
  let result = (seq char a e | str join '|')
  assert ($result == )
}


