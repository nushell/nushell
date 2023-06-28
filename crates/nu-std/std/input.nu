# Display user interface events
# 
# Press escape to stop
#
# To get individual events as records use "input listen"
export def display [
  --types(-t): list<string> # Listen for event of specified types only (can be one of: focus, key, mouse, paste, resize)
  --raw # Add raw_code field with numeric value of keycode and raw_flags with bit mask flags
] {
  let types = if $types == null { 'null' } else { $types }
  loop {
    let next_key = match [$types $raw] {
      ['null' false] => (input listen)
      ['null' true] => (input listen --raw)
      [$t false] => (input listen -t $t)
      [$t true] => (input listen -t $t --raw)
    }

    match $next_key {
      {type: key key_type: other code: esc modifiers: []} => {
        return
      }
      _ => {
        $next_key | table -e | print
      }
    }
  }
}
