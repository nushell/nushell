use std assert

# Parameter name:
# sig type   : any
# name       : variable
# type       : positional
# shape      : any
# description: the environment variable to temporarily set

# Parameter name:
# sig type   : any
# name       : block
# type       : positional
# shape      : closure()
# description: the block to run once the variable is set


# This is the custom command 1 for with-env:

#[test]
def with-env_set_the_myenv_environment_variable_1 [] {
  let result = (with-env [MYENV "my env value"] { $env.MYENV })
  assert ($result == my env value)
}

# This is the custom command 2 for with-env:

#[test]
def with-env_set_by_primitive_value_list_2 [] {
  let result = (with-env [X Y W Z] { $env.X })
  assert ($result == Y)
}

# This is the custom command 3 for with-env:

#[test]
def with-env_set_by_single_row_table_3 [] {
  let result = (with-env [[X W]; [Y Z]] { $env.W })
  assert ($result == Z)
}

# This is the custom command 4 for with-env:

#[test]
def with-env_set_by_key_value_record_4 [] {
  let result = (with-env {X: "Y", W: "Z"} { [$env.X $env.W] })
  assert ($result == [Y, Z])
}


