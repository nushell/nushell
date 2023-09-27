use std assert

# Parameter name:
# sig type   : any
# name       : no-newline
# type       : switch
# shape      : 
# description: print without inserting a newline for the line ending

# Parameter name:
# sig type   : any
# name       : stderr
# type       : switch
# shape      : 
# description: print to stderr instead of stdout

# Parameter name:
# sig type   : nothing
# name       : no-newline
# type       : switch
# shape      : 
# description: print without inserting a newline for the line ending

# Parameter name:
# sig type   : nothing
# name       : stderr
# type       : switch
# shape      : 
# description: print to stderr instead of stdout


# This is the custom command 1 for print:

#[test]
def print_print_hello_world_1 [] {
  let result = (print "hello world")
  assert ($result == )
}

# This is the custom command 2 for print:

#[test]
def print_print_the_sum_of_2_and_3_2 [] {
  let result = (print (2 + 3))
  assert ($result == )
}


