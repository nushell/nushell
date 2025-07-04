export def default_completion_menu [] {{
  name: completion_menu
  only_buffer_difference: false
  marker: "| "
  type: {
      layout: columnar
      columns: 4
      col_width: 20
      col_padding: 2
  }
  style: {
      text: green,
      selected_text: green_reverse
      description_text: yellow
  }
}}

export def default_ide_completion_menu [] {{
  name: ide_completion_menu
  only_buffer_difference: false
  marker: "| "
  type: {
    layout: ide
    min_completion_width: 0,
    max_completion_width: 50,
    max_completion_height: 10, # will be limited by the available lines in the terminal
    padding: 0,
    border: true,
    cursor_offset: 0,
    description_mode: "prefer_right"
    min_description_width: 0
    max_description_width: 50
    max_description_height: 10
    description_offset: 1
    # If true, the cursor pos will be corrected, so the suggestions match up with the typed text
    #
    # C:\> str
    #      str join
    #      str trim
    #      str split
    correct_cursor_pos: false
  }
  style: {
    text: green
    selected_text: { attr: r }
    description_text: yellow
    match_text: { attr: u }
    selected_match_text: { attr: ur }
  }
}}

export def default_history_menu [] {{
  name: history_menu
  only_buffer_difference: true
  marker: "? "
  type: {
      layout: list
      page_size: 10
  }
  style: {
      text: green,
      selected_text: green_reverse
      description_text: yellow
  }
}}

export def default_help_menu [] {{
  name: help_menu
  only_buffer_difference: true
  marker: "? "
  type: {
      layout: description
      columns: 4
      col_width: 20
      col_padding: 2
      selection_rows: 4
      description_rows: 10
  }
  style: {
      text: green,
      selected_text: green_reverse
      description_text: yellow
  }
}}
