use std assert

# Parameter name:
# sig type   : list<string>
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, remove ANSI sequences from strings at the given cell paths

# Parameter name:
# sig type   : record
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, remove ANSI sequences from strings at the given cell paths

# Parameter name:
# sig type   : string
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, remove ANSI sequences from strings at the given cell paths

# Parameter name:
# sig type   : table
# name       : cell path
# type       : rest
# shape      : cell-path
# description: for a data structure input, remove ANSI sequences from strings at the given cell paths


# This is the custom command 1 for ansi_strip:

#[test]
def ansi_strip_strip_ansi_escape_sequences_from_a_string_1 [] {
  let result = ($'(ansi green)(ansi cursor_on)hello' | ansi strip)
  assert ($result == "hello")
}


