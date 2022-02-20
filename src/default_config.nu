# Nushell Config File

def create_left_prompt [] {
    let path_segment = ([
        ($nu.cwd)
        (char space)
    ] | str collect)

    $path_segment
}

def create_right_prompt [] {
    let time_segment = ([
        (date now | date format '%m/%d/%Y %I:%M:%S%.3f')
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
  filesize_metric: $true
  table_mode: rounded # basic, compact, compact_double, light, thin, with_love, rounded, reinforced, heavy, none, other
  use_ls_colors: $true
  rm_always_trash: $false
  color_config: {
    separator: yd
    leading_trailing_space_bg: white
    header: cb
    date: pu
    filesize: ub
    row_index: yb
    hints: dark_gray
    bool: red
    int: green
    duration: red
    range: red
    float: red
    string: red
    nothing: red
    binary: red
    cellpath: red
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
    text_style: red
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
