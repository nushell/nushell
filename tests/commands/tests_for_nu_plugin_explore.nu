use std assert

# Parameter name:
# sig type   : any
# name       : config
# type       : positional
# shape      : record
# description: a config record to configure everything in explore


# This is the custom command 1 for nu_plugin_explore:

#[test]
def nu_plugin_explore_explore_the_cargotoml_file_of_this_project_1 [] {
  let result = (open Cargo.toml | explore)
  assert ($result == )
}

# This is the custom command 2 for nu_plugin_explore:

#[test]
def nu_plugin_explore_explore_nu_and_set_some_config_options_2 [] {
  let result = ($nu | explore {show_cell_path: false, layout: "compact"})
  assert ($result == )
}


