use std assert

# Parameter name:
# sig type   : nothing
# name       : pid
# type       : positional
# shape      : int
# description: process id of process that is to be killed

# Parameter name:
# sig type   : nothing
# name       : force
# type       : switch
# shape      : 
# description: forcefully kill the process

# Parameter name:
# sig type   : nothing
# name       : quiet
# type       : switch
# shape      : 
# description: won't print anything to the console


# This is the custom command 1 for kill:

#[test]
def kill_kill_the_pid_using_the_most_memory_1 [] {
  let result = (ps | sort-by mem | last | kill $in.pid)
  assert ($result == )
}

# This is the custom command 2 for kill:

#[test]
def kill_force_kill_a_given_pid_2 [] {
  let result = (kill --force 12345)
  assert ($result == )
}


