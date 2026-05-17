use std/testing *
use std/assert

@test
def std_pre_import [] {
  # These commands shouldn't exist without an import
  assert length (scope commands | where name == "path add") 0
  assert length (scope commands | where name == "ellie") 0
  assert length (scope commands | where name == "repeat") 0
  assert length (scope commands | where name == "from jsonl") 0
  assert length (scope commands | where name == "datetime-diff") 0
}
