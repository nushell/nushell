use std assert

# Parameter name:
# sig type   : nothing
# name       : long
# type       : switch
# shape      : 
# description: list all available columns for each entry


# This is the custom command 1 for ps:

#[test]
def ps_list_the_system_processes_1 [] {
  let result = (ps)
  assert ($result == )
}

# This is the custom command 2 for ps:

#[test]
def ps_list_the_top_5_system_processes_with_the_highest_memory_usage_2 [] {
  let result = (ps | sort-by mem | last 5)
  assert ($result == )
}

# This is the custom command 3 for ps:

#[test]
def ps_list_the_top_3_system_processes_with_the_highest_cpu_usage_3 [] {
  let result = (ps | sort-by cpu | last 3)
  assert ($result == )
}

# This is the custom command 4 for ps:

#[test]
def ps_list_the_system_processes_with_nu_in_their_names_4 [] {
  let result = (ps | where name =~ 'nu')
  assert ($result == )
}

# This is the custom command 5 for ps:

#[test]
def ps_get_the_parent_process_id_of_the_current_nu_process_5 [] {
  let result = (ps | where pid == $nu.pid | get ppid)
  assert ($result == )
}


