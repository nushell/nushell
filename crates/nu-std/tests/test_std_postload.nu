use std/testing *
use std/assert
export use std *

@test
def std_post_import [] {
  assert length (scope commands | where name == "path add") 1
  assert length (scope commands | where name == "ellie") 1
  assert length (scope commands | where name == "repeat") 1
  assert length (scope commands | where name == "formats from jsonl") 1
  assert length (scope commands | where name == "dt datetime-diff") 1
}
