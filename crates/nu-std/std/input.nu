# Display user interface events
# 
# Press escape to stop
#
# To get individual events as records use "input listen"
export def display [
  --types(-t): list<string> # Listen for event of specified types only (can be one of: focus, key, mouse, paste, resize)
  --raw # Add raw_code field with numeric value of keycode and raw_flags with bit mask flags
] {
  let arg_types = if $types == null {
    [ key ]
  } else if 'key' not-in $types {
    $types | append 'key'
  } else {
    $types
  }

  # To get exit key 'escape' we need to read key 
  # type events, hovever user may filter them out 
  # using --types and they should not be displayed
  let filter_keys = 'key' not-in $types
  
  loop {
    let next_key = if $raw {
      input listen -t $arg_types -r  
    } else {
      input listen -t $arg_types
    }

    match $next_key {
      {type: key key_type: other code: esc modifiers: []} => {
        return
      }
      _ => {
        if (not $filter_keys) or $next_key.type != 'key' {
          $next_key | table -e | print
        }
      }
    }
  }
}
