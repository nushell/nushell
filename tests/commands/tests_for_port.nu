use std assert

# Parameter name:
# sig type   : nothing
# name       : start
# type       : positional
# shape      : int
# description: The start port to scan (inclusive)

# Parameter name:
# sig type   : nothing
# name       : end
# type       : positional
# shape      : int
# description: The end port to scan (inclusive)


# This is the custom command 1 for port:

#[test]
def port_get_a_free_port_between_3121_and_4000_1 [] {
  let result = (port 3121 4000)
  assert ($result == 3121)
}

# This is the custom command 2 for port:

#[test]
def port_get_a_free_port_from_system_2 [] {
  let result = (port)
  assert ($result == )
}


