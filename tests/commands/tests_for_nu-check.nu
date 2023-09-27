use std assert

# Parameter name:
# sig type   : list<any>
# name       : path
# type       : positional
# shape      : string
# description: File path to parse

# Parameter name:
# sig type   : list<any>
# name       : as-module
# type       : switch
# shape      : 
# description: Parse content as module

# Parameter name:
# sig type   : list<any>
# name       : debug
# type       : switch
# shape      : 
# description: Show error messages

# Parameter name:
# sig type   : list<any>
# name       : all
# type       : switch
# shape      : 
# description: Parse content as script first, returns result if success, otherwise, try with module

# Parameter name:
# sig type   : string
# name       : path
# type       : positional
# shape      : string
# description: File path to parse

# Parameter name:
# sig type   : string
# name       : as-module
# type       : switch
# shape      : 
# description: Parse content as module

# Parameter name:
# sig type   : string
# name       : debug
# type       : switch
# shape      : 
# description: Show error messages

# Parameter name:
# sig type   : string
# name       : all
# type       : switch
# shape      : 
# description: Parse content as script first, returns result if success, otherwise, try with module


# This is the custom command 1 for nu-check:

#[test]
def nu-check_parse_a_input_file_as_scriptdefault_1 [] {
  let result = (nu-check script.nu)
  assert ($result == )
}

# This is the custom command 2 for nu-check:

#[test]
def nu-check_parse_a_input_file_as_module_2 [] {
  let result = (nu-check --as-module module.nu)
  assert ($result == )
}

# This is the custom command 3 for nu-check:

#[test]
def nu-check_parse_a_input_file_by_showing_error_message_3 [] {
  let result = (nu-check -d script.nu)
  assert ($result == )
}

# This is the custom command 4 for nu-check:

#[test]
def nu-check_parse_an_external_stream_as_script_by_showing_error_message_4 [] {
  let result = (open foo.nu | nu-check -d script.nu)
  assert ($result == )
}

# This is the custom command 5 for nu-check:

#[test]
def nu-check_parse_an_internal_stream_as_module_by_showing_error_message_5 [] {
  let result = (open module.nu | lines | nu-check -d --as-module module.nu)
  assert ($result == )
}

# This is the custom command 6 for nu-check:

#[test]
def nu-check_parse_a_string_as_script_6 [] {
  let result = ($'two(char nl)lines' | nu-check )
  assert ($result == )
}

# This is the custom command 7 for nu-check:

#[test]
def nu-check_heuristically_parse_which_begins_with_script_first_if_it_sees_a_failure_try_module_afterwards_7 [] {
  let result = (nu-check -a script.nu)
  assert ($result == )
}

# This is the custom command 8 for nu-check:

#[test]
def nu-check_heuristically_parse_by_showing_error_message_8 [] {
  let result = (open foo.nu | lines | nu-check -ad)
  assert ($result == )
}


