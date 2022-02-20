# Nushell Config File

def create_left_prompt [] {
    let path_segment = ($nu.cwd)

    $path_segment
}

def create_right_prompt [] {
    let time_segment = ([
        (date now | date format '%m/%d/%Y %r')
    ] | str collect)

    $time_segment
}

# Use nushell functions to define your right and left prompt
let-env PROMPT_COMMAND = { create_left_prompt }
let-env PROMPT_COMMAND_RIGHT = { create_right_prompt }

# The prompt indicators are environmental variables that represent
# the state of the prompt
let-env PROMPT_INDICATOR = "〉"
let-env PROMPT_INDICATOR_VI_INSERT = ": "
let-env PROMPT_INDICATOR_VI_NORMAL = "〉"
let-env PROMPT_MULTILINE_INDICATOR = "::: "

let $config = {
  filesize_metric: $false
  table_mode: rounded # basic, compact, compact_double, light, thin, with_love, rounded, reinforced, heavy, none, other
  use_ls_colors: $true
  rm_always_trash: $false
  color_config: {
    separator: white
    leading_trailing_space_bg: white
    header: green_bold
    date: white
    filesize: white
    row_index: green_bold
    hints: dark_gray
    bool: white
    int: white
    duration: white
    range: white
    float: white
    string: white
    nothing: white
    binary: white
    cellpath: white
  }
  use_grid_icons: $true
  footer_mode: always #always, never, number_of_rows, auto
  quick_completions: $false
  animate_prompt: $false
  float_precision: 2
  use_ansi_coloring: $true
  filesize_format: "b" # b, kb, kib, mb, mib, gb, gib, tb, tib, pb, pib, eb, eib, zb, zib, auto
  env_conversions: {
    "PATH": {
        from_string: { |s| $s | split row (char esep) }
        to_string: { |v| $v | str collect (char esep) }
        }
  }
  edit_mode: emacs # emacs, vi
  max_history_size: 10000
  menu_config: {
    columns: 4
    col_width: 20   # Optional value. If missing all the screen width is used to calculate column width
    col_padding: 2
    text_style: green
    selected_text_style: green_reverse
    marker: "| "
  }
  history_config: {
   page_size: 10
   selector: ":"                                                                                                                          
   text_style: green
   selected_text_style: green_reverse
   marker: "? "
  }
  keybindings: [
    {
      name: completion
      modifier: control
      keycode: char_t
      mode: vi_insert # emacs vi_normal vi_insert
      event: { send: menu name: context_menu }
    }
  ]
}
