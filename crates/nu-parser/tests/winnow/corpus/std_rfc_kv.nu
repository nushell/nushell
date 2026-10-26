const default_table_name = 'std_kv_store'
const valid_table_regex = '^[a-zA-Z0-9_]+$'
# kv module
#
# use std-rfc/kv *
#
# Easily store and retrieve key-value pairs
# in a pipeline.
#
# A common request is to be able to assign a
# pipeline result to a variable. While it's
# not currently possible to use a "let" statement
# within a pipeline, this module provides an
# alternative. Think of each key as a variable
# that can be set and retrieved.

# Stores the pipeline value for later use
#
# If the key already exists, it is updated to the new value provided.
@example "Store the list of files in the home directory" {
  ls ~ | kv set "home snapshot"
}
@example "Store a number" {
  kv set foo 5
}
@example "Store a number in a custom table named foo" {
  kv set -t foo bar 5
}
@example "Update a number and return it" {
  let $new_foo = (kv get foo | kv set foo { $in + 1 } --return value)
}
@example "Use a single pipeline with closures" {
  ls
  | kv set names { get name }
  | kv set sizes { get size }
}
export def "kv set" [
  key: string
  value_or_closure?: any
  --return (-r): string   # Whether and what to return to the pipeline output
  --universal (-u)        # Store the key-value pair in a universal database
  --table (-t): string = $default_table_name    # Optional table name
] {
  # Pipeline input is preferred, but prioritize
  # parameter if present. This allows $in to be
  # used in the parameter if needed.
  let input = $in

  # If passed a closure, execute it
  let arg_type = ($value_or_closure | describe)
  let value = match $arg_type {
    closure => { $input | do $value_or_closure }
    _ => ($value_or_closure | default $input)
  }

  # Store values as nuons for type-integrity
  let kv_pair = {
    session: ''   # Placeholder
    key: $key
    value: ($value | to nuon)
  }

  let db_open = (db_setup --universal=$universal --table=$table)
  try {
    # Delete the existing key if it does exist
    do $db_open | query db $"DELETE FROM ($table) WHERE key = :key" --params { key: $key }
  }

  match $universal {
    true  => { $kv_pair | into sqlite (universal_db_path) -t $table }
    false => { $kv_pair | stor insert -t $table }
  }

  # The value that should be returned from `kv set`
  # By default, this is the input to `kv set`, even if
  # overridden by a positional parameter.
  # This can also be:
  # input: (Default) The pipeline input to `kv set`, even if
  #        overridden by a positional parameter. `null` if no
  #        pipeline input was used.
  # ---
  # value: If a positional parameter was used for the value, then
  #        return it, otherwise return the input (whatever was set).
  #        If the positional was a closure, return the result of the
  #        closure on the pipeline input.
  # ---
  # all: The entire contents of the existing kv table are returned
  match ($return | default 'input') {
    'all' => (kv list --universal=$universal --table=$table)
    'a' => (kv list --universal=$universal --table=$table)
    'value' => $value
    'v' => $value
    'input' => $input
    'in' => $input
    'i' => $input
    _  => {
      error make {
        msg: "Invalid --return option"
        label: {
          text: "Must be 'all'/'a', 'value'/'v', or 'input'/'in'/'i'"
          span: (metadata $return).span
        }
      }
    }
  }
}

# Retrieves a stored value by key
#
# Counterpart of "kv set". Returns null if the key is not found.
@example "Retrieve a stored value" {
  kv get foo
}
export def "kv get" [
  key: string       # Key of the kv-pair to retrieve
  --universal (-u)  # Whether to use the universal db
  --table (-t): string = $default_table_name    # Optional table name
] {
  let db_open = (db_setup --universal=$universal --table=$table)
  do $db_open
    | query db $"SELECT value FROM ($table) WHERE key = :key" --params { key: $key }
    | match $in {
      # Match should be exactly one row
      [$el] => { $el.value | from nuon }
      # Otherwise no match
      _ => null
    }
}

# List the currently stored key-value pairs
#
# Returns results as the Nushell value rather than the stored nuon.
export def "kv list" [
  --universal (-u)  # Whether to use the universal db
  --table (-t): string = $default_table_name    # Optional table name
] {
  let db_open = (db_setup --universal=$universal --table=$table)
  do $db_open | $in | get -o $table | each {|kv_pair|
    {
      key: $kv_pair.key
      value: ($kv_pair.value | from nuon )
    }
  }
}

# Returns and removes a key-value pair
export def --env "kv drop" [
  key: string       # Key of the kv-pair to drop
  --universal (-u)  # Whether to use the universal db
  --table (-t): string = $default_table_name    # Optional table name
] {
  let db_open = (db_setup --universal=$universal --table=$table)

  let value = (kv get --universal=$universal --table=$table $key)

  try {
    do $db_open
      # Hack to turn a SQLiteDatabase into a table
      | query db $"DELETE FROM ($table) WHERE key = :key" --params { key: $key }
  }

  if $universal and ($env.NU_KV_UNIVERSALS? | default false) {
    hide-env $key
  }

  $value
}

def universal_db_path [] {
  $env.NU_UNIVERSAL_KV_PATH?
  | default (
    $nu.data-dir | path join "std_kv_variables.sqlite3"
  )
}

def db_setup [
  --universal       # Whether to use the universal db
  --table: string   # The table name to use
] : nothing -> closure {

  if $table not-like $valid_table_regex {
    error make {
      msg: "Invalid table name"
      label: {
        text: "Table name must match regex: ${valid_table_regex}"
        span: (metadata $table).span
      }
    }
  }

  try {
    match $universal {
      true  => {
        # Ensure universal sqlite db and table exists
        let uuid = (random uuid)
        let dummy_record = {
          session: ''
          key: $uuid
          value: ''
        }
        $dummy_record | into sqlite (universal_db_path) -t $table
        open (universal_db_path) | query db $"DELETE FROM ($table) WHERE key = :key" --params { key: $uuid }
      }
      false => {
        # Create the stor table if it doesn't exist
        stor create -t $table -c {session: str, key: str, value: str} | ignore
      }
    }
  }

  # Return the correct closure for opening on-disk vs. in-memory
  match $universal {
    true  => {{|| open (universal_db_path)}}
    false => {{|| stor open}}
  }
}

# This hook can be added to $env.config.hooks.pre_execution to enable
# "universal variables" similar to the Fish shell. Adding, changing, or
# removing a universal variable will immediately update the corresponding
# environment variable in all running Nushell sessions.
export def "kv universal-variable-hook" [] {
{||
  kv list --universal
  | transpose -dr
  | load-env

  $env.NU_KV_UNIVERSALS = true
}
}
