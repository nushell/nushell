use std assert


# This is the custom command 1 for sys:

#[test]
def sys_show_info_about_the_system_1 [] {
  let result = (sys)
  assert ($result == )
}

# This is the custom command 2 for sys:

#[test]
def sys_show_the_os_system_name_with_get_2 [] {
  let result = ((sys).host | get name)
  assert ($result == )
}

# This is the custom command 3 for sys:

#[test]
def sys_show_the_os_system_name_3 [] {
  let result = ((sys).host.name)
  assert ($result == )
}


