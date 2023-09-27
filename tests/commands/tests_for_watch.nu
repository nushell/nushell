use std assert

# Parameter name:
# sig type   : nothing
# name       : path
# type       : positional
# shape      : path
# description: the path to watch. Can be a file or directory

# Parameter name:
# sig type   : nothing
# name       : closure
# type       : positional
# shape      : closure(string, string, string)
# description: Some Nu code to run whenever a file changes. The closure will be passed `operation`, `path`, and `new_path` (for renames only) arguments in that order

# Parameter name:
# sig type   : nothing
# name       : debounce-ms
# type       : named
# shape      : int
# description: Debounce changes for this many milliseconds (default: 100). Adjust if you find that single writes are reported as multiple events

# Parameter name:
# sig type   : nothing
# name       : glob
# type       : named
# shape      : string
# description: Only report changes for files that match this glob pattern (default: all files)

# Parameter name:
# sig type   : nothing
# name       : recursive
# type       : named
# shape      : bool
# description: Watch all directories under `<path>` recursively. Will be ignored if `<path>` is a file (default: true)

# Parameter name:
# sig type   : nothing
# name       : verbose
# type       : switch
# shape      : 
# description: Operate in verbose mode (default: false)


# This is the custom command 1 for watch:

#[test]
def watch_run_cargo_test_whenever_a_rust_file_changes_1 [] {
  let result = (watch . --glob=**/*.rs {|| cargo test })
  assert ($result == )
}

# This is the custom command 2 for watch:

#[test]
def watch_watch_all_changes_in_the_current_directory_2 [] {
  let result = (watch . { |op, path, new_path| $"($op) ($path) ($new_path)"})
  assert ($result == )
}

# This is the custom command 3 for watch:

#[test]
def watch_log_all_changes_in_a_directory_3 [] {
  let result = (watch /foo/bar { |op, path| $"($op) - ($path)(char nl)" | save --append changes_in_bar.log })
  assert ($result == )
}

# This is the custom command 4 for watch:

#[test]
def watch_note_if_you_are_looking_to_run_a_command_every_n_units_of_time_this_can_be_accomplished_with_a_loop_and_sleep_4 [] {
  let result = (loop { command; sleep duration })
  assert ($result == )
}


