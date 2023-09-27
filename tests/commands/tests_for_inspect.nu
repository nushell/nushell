use std assert


# This is the custom command 1 for inspect:

#[test]
def inspect_inspect_pipeline_results_1 [] {
  let result = (ls | inspect | get name | inspect)
  assert ($result == )
}


