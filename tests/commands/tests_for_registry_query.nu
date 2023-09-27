use std assert

# Parameter name:
# sig type   : nothing
# name       : key
# type       : positional
# shape      : string
# description: registry key to query

# Parameter name:
# sig type   : nothing
# name       : value
# type       : positional
# shape      : string
# description: optionally supply a registry value to query

# Parameter name:
# sig type   : nothing
# name       : hkcr
# type       : switch
# shape      : 
# description: query the hkey_classes_root hive

# Parameter name:
# sig type   : nothing
# name       : hkcu
# type       : switch
# shape      : 
# description: query the hkey_current_user hive

# Parameter name:
# sig type   : nothing
# name       : hklm
# type       : switch
# shape      : 
# description: query the hkey_local_machine hive

# Parameter name:
# sig type   : nothing
# name       : hku
# type       : switch
# shape      : 
# description: query the hkey_users hive

# Parameter name:
# sig type   : nothing
# name       : hkpd
# type       : switch
# shape      : 
# description: query the hkey_performance_data hive

# Parameter name:
# sig type   : nothing
# name       : hkpt
# type       : switch
# shape      : 
# description: query the hkey_performance_text hive

# Parameter name:
# sig type   : nothing
# name       : hkpnls
# type       : switch
# shape      : 
# description: query the hkey_performance_nls_text hive

# Parameter name:
# sig type   : nothing
# name       : hkcc
# type       : switch
# shape      : 
# description: query the hkey_current_config hive

# Parameter name:
# sig type   : nothing
# name       : hkdd
# type       : switch
# shape      : 
# description: query the hkey_dyn_data hive

# Parameter name:
# sig type   : nothing
# name       : hkculs
# type       : switch
# shape      : 
# description: query the hkey_current_user_local_settings hive


# This is the custom command 1 for registry_query:

#[test]
def registry_query_query_the_hkey_current_user_hive_1 [] {
  let result = (registry query --hkcu environment)
  assert ($result == )
}

# This is the custom command 2 for registry_query:

#[test]
def registry_query_query_the_hkey_local_machine_hive_2 [] {
  let result = (registry query --hklm 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment')
  assert ($result == )
}


