use std assert

# Parameter name:
# sig type   : nothing
# name       : plugin
# type       : positional
# shape      : path
# description: path of executable for plugin

# Parameter name:
# sig type   : nothing
# name       : signature
# type       : positional
# shape      : any
# description: Block with signature description as json object

# Parameter name:
# sig type   : nothing
# name       : shell
# type       : named
# shape      : path
# description: path of shell used to run plugin (cmd, sh, python, etc)


# This is the custom command 1 for register:

#[test]
def register_register_nu_plugin_query_plugin_from_cargobin_dir_1 [] {
  let result = (register ~/.cargo/bin/nu_plugin_query)
  assert ($result == )
}

# This is the custom command 2 for register:

#[test]
def register_register_nu_plugin_query_plugin_from_nu__c_writesupdates_nuplugin_path_2 [] {
  let result = (let plugin = ((which nu).path.0 | path dirname | path join 'nu_plugin_query'); nu -c $'register ($plugin); version')
  assert ($result == )
}


