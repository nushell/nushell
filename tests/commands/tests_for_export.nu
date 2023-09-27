use std assert


# This is the custom command 1 for export:

#[test]
def export_export_a_definition_from_a_module_1 [] {
  let result = (module utils { export def my-command [] { "hello" } }; use utils my-command; my-command)
  assert ($result == hello)
}


