use std/assert

#[test]
def std_pre_import [] {
  # These commands shouldn't exist without an import
  assert length (scope commands | where name == "path add") 0
  assert length (scope commands | where name == "ellie") 0
  assert length (scope commands | where name == "repeat") 0
  assert length (scope commands | where name == "from jsonl") 0
  assert length (scope commands | where name == "datetime-diff") 0
}

def std_post_import [] {
  # After importing std, these commands should be
  # available at the top level namespace
  use std *
  assert length (scope commands | where name == "path add") 1
  assert length (scope commands | where name == "ellie") 1
  assert length (scope commands | where name == "repeat") 1
  assert length (scope commands | where name == "from jsonl") 1
  assert length (scope commands | where name == "datetime-diff") 1
}