use std assert


# This is the custom command 1 for seq:

#[test]
def seq_sequence_1_to_10_1 [] {
  let result = (seq 1 10)
  assert ($result == [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
}

# This is the custom command 2 for seq:

#[test]
def seq_sequence_10_to_20_by_01s_2 [] {
  let result = (seq 1.0 0.1 2.0)
  assert ($result == [1, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2])
}

# This is the custom command 3 for seq:

#[test]
def seq_sequence_1_to_5_then_convert_to_a_string_with_a_pipe_separator_3 [] {
  let result = (seq 1 5 | str join '|')
  assert ($result == )
}


