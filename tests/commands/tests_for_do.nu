use std assert

# Parameter name:
# sig type   : any
# name       : closure
# type       : positional
# shape      : one_of(closure(), any)
# description: the closure to run

# Parameter name:
# sig type   : any
# name       : ignore-errors
# type       : switch
# shape      : 
# description: ignore errors as the closure runs

# Parameter name:
# sig type   : any
# name       : ignore-shell-errors
# type       : switch
# shape      : 
# description: ignore shell errors as the closure runs

# Parameter name:
# sig type   : any
# name       : ignore-program-errors
# type       : switch
# shape      : 
# description: ignore external program errors as the closure runs

# Parameter name:
# sig type   : any
# name       : capture-errors
# type       : switch
# shape      : 
# description: catch errors as the closure runs, and return them


# This is the custom command 1 for do:

#[test]
def do_run_the_closure_1 [] {
  let result = (do { echo hello })
  assert ($result == hello)
}

# This is the custom command 2 for do:

#[test]
def do_run_a_stored_first_class_closure_2 [] {
  let result = (let text = "I am enclosed"; let hello = {|| echo $text}; do $hello)
  assert ($result == I am enclosed)
}

# This is the custom command 3 for do:

#[test]
def do_run_the_closure_and_ignore_both_shell_and_external_program_errors_3 [] {
  let result = (do -i { thisisnotarealcommand })
  assert ($result == )
}

# This is the custom command 4 for do:

#[test]
def do_run_the_closure_and_ignore_shell_errors_4 [] {
  let result = (do -s { thisisnotarealcommand })
  assert ($result == )
}

# This is the custom command 5 for do:

#[test]
def do_run_the_closure_and_ignore_external_program_errors_5 [] {
  let result = (do -p { nu -c 'exit 1' }; echo "I'll still run")
  assert ($result == )
}

# This is the custom command 6 for do:

#[test]
def do_abort_the_pipeline_if_a_program_returns_a_non_zero_exit_code_6 [] {
  let result = (do -c { nu -c 'exit 1' } | myscarycommand)
  assert ($result == )
}

# This is the custom command 7 for do:

#[test]
def do_run_the_closure_with_a_positional_parameter_7 [] {
  let result = (do {|x| 100 + $x } 77)
  assert ($result == 177)
}

# This is the custom command 8 for do:

#[test]
def do_run_the_closure_with_input_8 [] {
  let result = (77 | do {|x| 100 + $in })
  assert ($result == )
}


