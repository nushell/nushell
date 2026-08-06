//! Example JSON data for testing the explore config TUI.

use serde_json::Value;

/// Example JSON data for testing (nushell config)
#[allow(dead_code)]
pub fn get_example_json() -> Value {
    let json_str = r##"{
  "filesize": {
    "unit": "B",
    "show_unit": false,
    "precision": 2
  },
  "table": {
    "mode": "rounded",
    "index_mode": "always",
    "show_empty": false,
    "padding": {
      "left": 1,
      "right": 1
    },
    "trim": {
      "methodology": "wrapping",
      "wrapping_try_keep_words": true
    },
    "header_on_separator": true,
    "abbreviated_row_count": null,
    "footer_inheritance": true,
    "missing_value_symbol": "❎",
    "batch_duration": 1000000000,
    "stream_page_size": 1000
  },
  "ls": {
    "use_ls_colors": true,
    "clickable_links": true
  },
  "color_config": {
    "shape_internallcall": "cyan_bold",
    "leading_trailing_space_bg": {
      "bg": "dark_gray_dimmed"
    },
    "string": "{|x| if $x =~ '^#[a-fA-F\\d]+' { $x } else { 'default' } }",
    "date": "{||\n    (date now) - $in | if $in < 1hr {\n      'red3b' #\"\\e[38;5;160m\" #'#e61919' # 160\n    } else if $in < 6hr {\n      'orange3' #\"\\e[38;5;172m\" #'#e68019' # 172\n    } else if $in < 1day {\n      'yellow3b' #\"\\e[38;5;184m\" #'#e5e619' # 184\n    } else if $in < 3day {\n      'chartreuse2b' #\"\\e[38;5;112m\" #'#80e619' # 112\n    } else if $in < 1wk {\n      'green3b' #\"\\e[38;5;40m\" #'#19e619' # 40\n    } else if $in < 6wk {\n      'darkturquoise' #\"\\e[38;5;44m\" #'#19e5e6' # 44\n    } else if $in < 52wk {\n      'deepskyblue3b' #\"\\e[38;5;32m\" #'#197fe6' # 32\n    } else { 'dark_gray' }\n  }",
    "hints": "dark_gray",
    "shape_matching_brackets": {
      "fg": "red",
      "bg": "default",
      "attr": "b"
    },
    "nothing": "red",
    "shape_string_interpolation": "cyan_bold",
    "shape_externalarg": "light_purple",
    "shape_external_resolved": "light_yellow_bold",
    "cellpath": "cyan",
    "foreground": "green3b",
    "shape_filepath": "cyan",
    "separator": "yd",
    "shape_garbage": {
      "fg": "red",
      "attr": "u"
    },
    "shape_external": "darkorange",
    "float": "red",
    "shape_block": "#33ff00",
    "shape_bool": "{|| if $in { 'light_cyan' } else { 'light_red' } }",
    "binary": "red",
    "duration": "blue_bold",
    "header": "cb",
    "filesize": "{|e| if $e == 0b { 'black' } else if $e < 1mb { 'ub' } else { 'cyan' } }",
    "range": "purple",
    "search_result": "blue_reverse",
    "bool": "{|| if $in { 'light_cyan' } else { 'light_red' } }",
    "int": "green",
    "row_index": "yb",
    "shape_closure": "#ffb000"
  },
  "footer_mode": "auto",
  "float_precision": 2,
  "recursion_limit": 50,
  "use_ansi_coloring": "true",
  "completions": {
    "sort": "smart",
    "case_sensitive": false,
    "quick": true,
    "partial": true,
    "algorithm": "prefix",
    "external": {
      "enable": true,
      "max_results": 10,
      "completer": null
    },
    "use_ls_colors": true
  },
  "edit_mode": "emacs",
  "history": {
    "max_size": 1000000,
    "sync_on_enter": true,
    "file_format": "sqlite",
    "isolation": true
  },
  "keybindings": [
    {
      "name": "open_command_editor",
      "modifier": "control",
      "keycode": "char_o",
      "event": {
        "send": "openeditor"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "clear_everything",
      "modifier": "control",
      "keycode": "char_l",
      "event": [
        {
          "send": "clearscrollback"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "insert_newline",
      "modifier": "shift",
      "keycode": "enter",
      "event": {
        "edit": "insertnewline"
      },
      "mode": "emacs"
    },
    {
      "name": "completion_menu",
      "modifier": "none",
      "keycode": "tab",
      "event": {
        "until": [
          {
            "send": "menu",
            "name": "completion_menu"
          },
          {
            "send": "menunext"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "completion_previous",
      "modifier": "shift",
      "keycode": "backtab",
      "event": {
        "send": "menuprevious"
      },
      "mode": "emacs"
    },
    {
      "name": "insert_last_token",
      "modifier": "alt",
      "keycode": "char_.",
      "event": [
        {
          "edit": "insertstring",
          "value": " !$"
        },
        {
          "send": "enter"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "complete_hint_chunk",
      "modifier": "alt",
      "keycode": "right",
      "event": {
        "until": [
          {
            "send": "historyhintwordcomplete"
          },
          {
            "edit": "movewordright"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "un_complete_hint_chunk",
      "modifier": "alt",
      "keycode": "left",
      "event": [
        {
          "edit": "backspaceword"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "delete-word",
      "modifier": "control",
      "keycode": "backspace",
      "event": {
        "until": [
          {
            "edit": "backspaceword"
          }
        ]
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "trigger-history-menu",
      "modifier": "control",
      "keycode": "char_x",
      "event": {
        "until": [
          {
            "send": "menu",
            "name": "history_menu"
          },
          {
            "send": "menupagenext"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "trigger-history-previous",
      "modifier": "control",
      "keycode": "char_z",
      "event": {
        "until": [
          {
            "send": "menupageprevious"
          },
          {
            "edit": "undo"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "change_dir_with_fzf",
      "modifier": "control",
      "keycode": "char_f",
      "event": {
        "send": "executehostcommand",
        "cmd": "cd (ls | where type == dir | each { |it| $it.name} | str join (char nl) | fzf | decode utf-8 | str trim)"
      },
      "mode": "emacs"
    },
    {
      "name": "complete_in_cd",
      "modifier": "none",
      "keycode": "f2",
      "event": [
        {
          "edit": "clear"
        },
        {
          "edit": "insertString",
          "value": "./"
        },
        {
          "send": "Menu",
          "name": "completion_menu"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "reload_config",
      "modifier": "none",
      "keycode": "f5",
      "event": [
        {
          "edit": "clear"
        },
        {
          "send": "executehostcommand",
          "cmd": "source C:\\Users\\username\\AppData\\Roaming\\nushell\\env.nu; source C:\\Users\\username\\AppData\\Roaming\\nushell\\config.nu"
        }
      ],
      "mode": [
        "emacs",
        "vi_insert",
        "vi_normal"
      ]
    },
    {
      "name": "clear",
      "modifier": "none",
      "keycode": "esc",
      "event": {
        "edit": "clear"
      },
      "mode": "emacs"
    },
    {
      "name": "test_fkeys",
      "modifier": "none",
      "keycode": "f3",
      "event": [
        {
          "edit": "clear"
        },
        {
          "edit": "insertstring",
          "value": "C:\\Users\\username\\source\\repos\\forks\\nushell"
        }
      ],
      "mode": "emacs"
    },
    {
      "name": "abbr",
      "modifier": "control",
      "keycode": "space",
      "event": [
        {
          "send": "menu",
          "name": "abbr_menu"
        },
        {
          "edit": "insertchar",
          "value": " "
        }
      ],
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "fzf_edit",
      "modifier": "control",
      "keycode": "char_d",
      "event": [
        {
          "send": "executehostcommand",
          "cmd": "do { |$file| if (not ($file | is-empty)) { nvim $file } } (fzf | str trim)"
        }
      ],
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "history_menu_by_session",
      "modifier": "alt",
      "keycode": "char_r",
      "event": {
        "send": "menu",
        "name": "history_menu_by_session"
      },
      "mode": "emacs"
    },
    {
      "name": "fuzzy_history",
      "modifier": "control",
      "keycode": "char_r",
      "event": [
        {
          "send": "ExecuteHostCommand",
          "cmd": "do {\n          $env.SHELL = 'c:/progra~1/git/usr/bin/bash.exe'\n          commandline edit -r (\n            history\n            | get command\n            | reverse\n            | uniq\n            | str join (char -i 0)\n            | fzf --scheme=history --read0 --layout=reverse --height=40% --bind 'tab:change-preview-window(right,70%|right)' -q (commandline) --preview='echo -n {} | nu --stdin -c 'nu-highlight''\n            | decode utf-8\n            | str trim\n          )\n        }"
        }
      ],
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "fuzzy_dir",
      "modifier": "control",
      "keycode": "char_s",
      "event": {
        "send": "executehostcommand",
        "cmd": "commandline edit -a (ls **/* | where type == dir | get name | to text | fzf -q (commandline) | str trim);commandline set-cursor --end"
      },
      "mode": "emacs"
    },
    {
      "name": "fzf_dir_menu_nu_ui",
      "modifier": "control",
      "keycode": "char_n",
      "event": {
        "send": "menu",
        "name": "fzf_dir_menu_nu_ui"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "fzf_history_menu_fzf_ui",
      "modifier": "control",
      "keycode": "char_e",
      "event": {
        "send": "menu",
        "name": "fzf_history_menu_fzf_ui"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "fzf_history_menu_nu_ui",
      "modifier": "control",
      "keycode": "char_w",
      "event": {
        "send": "menu",
        "name": "fzf_history_menu_nu_ui"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "commands_menu",
      "modifier": "control",
      "keycode": "char_t",
      "event": {
        "send": "menu",
        "name": "commands_menu"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "vars_menu",
      "modifier": "control",
      "keycode": "char_y",
      "event": {
        "send": "menu",
        "name": "vars_menu"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "commands_with_description",
      "modifier": "control",
      "keycode": "char_u",
      "event": {
        "send": "menu",
        "name": "commands_with_description"
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "trigger-help-menu",
      "modifier": "control",
      "keycode": "char_q",
      "event": {
        "until": [
          {
            "send": "menu",
            "name": "help_menu"
          },
          {
            "send": "menunext"
          }
        ]
      },
      "mode": "emacs"
    },
    {
      "name": "copy_selection",
      "modifier": "control_shift",
      "keycode": "char_c",
      "event": {
        "edit": "copyselection"
      },
      "mode": "emacs"
    },
    {
      "name": "cut_selection",
      "modifier": "control_shift",
      "keycode": "char_x",
      "event": {
        "edit": "cutselection"
      },
      "mode": "emacs"
    },
    {
      "name": "select_all",
      "modifier": "control_shift",
      "keycode": "char_a",
      "event": {
        "edit": "selectall"
      },
      "mode": "emacs"
    },
    {
      "name": "paste",
      "modifier": "control_shift",
      "keycode": "char_v",
      "event": {
        "edit": "pastecutbufferbefore"
      },
      "mode": "emacs"
    },
    {
      "name": "ide_completion_menu",
      "modifier": "control",
      "keycode": "char_n",
      "event": {
        "until": [
          {
            "send": "menu",
            "name": "ide_completion_menu"
          },
          {
            "send": "menunext"
          },
          {
            "edit": "complete"
          }
        ]
      },
      "mode": [
        "emacs",
        "vi_normal",
        "vi_insert"
      ]
    },
    {
      "name": "quick_assign",
      "modifier": "alt",
      "keycode": "char_a",
      "event": [
        {
          "edit": "MoveToStart"
        },
        {
          "edit": "InsertString",
          "value": "let foo = "
        },
        {
          "edit": "MoveLeftBefore",
          "value": "o"
        },
        {
          "edit": "MoveLeftUntil",
          "value": "f",
          "select": true
        }
      ],
      "mode": [
        "emacs",
        "vi_insert",
        "vi_normal"
      ]
    }
  ],
  "menus": [
    {
      "name": "ide_completion_menu",
      "marker": " \n❯ 📎 ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": {
          "attr": "r"
        },
        "description_text": "yellow",
        "match_text": {
          "fg": "#33ff00"
        },
        "selected_match_text": {
          "fg": "#33ff00",
          "attr": "r"
        }
      },
      "type": {
        "layout": "ide",
        "min_completion_width": 0,
        "max_completion_width": 50,
        "padding": 0,
        "border": true,
        "cursor_offset": 0,
        "description_mode": "prefer_right",
        "min_description_width": 0,
        "max_description_width": 50,
        "max_description_height": 10,
        "description_offset": 1,
        "correct_cursor_pos": true
      },
      "source": null
    },
    {
      "name": "completion_menu",
      "marker": " \n❯ 📎 ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": {
          "attr": "r"
        },
        "description_text": "yellow",
        "match_text": {
          "fg": "#33ff00"
        },
        "selected_match_text": {
          "fg": "#33ff00",
          "attr": "r"
        }
      },
      "type": {
        "layout": "columnar",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2,
        "tab_traversal": "vertical"
      },
      "source": null
    },
    {
      "name": "history_menu",
      "marker": "🔍 ",
      "only_buffer_difference": false,
      "style": {
        "text": "#ffb000",
        "selected_text": {
          "fg": "#ffb000",
          "attr": "r"
        },
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": null
    },
    {
      "name": "help_menu",
      "marker": "? ",
      "only_buffer_difference": true,
      "style": {
        "text": "#7F00FF",
        "selected_text": {
          "fg": "#ffff00",
          "bg": "#7F00FF",
          "attr": "b"
        },
        "description_text": "#ffff00"
      },
      "type": {
        "layout": "description",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2,
        "selection_rows": 4,
        "description_rows": 10
      },
      "source": null
    },
    {
      "name": "fzf_history_menu_fzf_ui",
      "marker": "# ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "columnar",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2
      },
      "source": "{|buffer, position|\n        open $nu.history-path | get history.command_line | to text | fzf +s --tac | str trim\n        | where $it =~ $buffer\n        | each {|v| {value: ($v | str trim)} }\n      }"
    },
    {
      "name": "fzf_history_menu_nu_ui",
      "marker": "# ",
      "only_buffer_difference": false,
      "style": {
        "text": "#66ff66",
        "selected_text": {
          "fg": "#66ff66",
          "attr": "r"
        },
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": "{|buffer, position|\n        open $nu.history-path | get history.command_line | to text\n        | fzf -f $buffer\n        | lines\n        | each {|v| {value: ($v | str trim)} }\n      }"
    },
    {
      "name": "fzf_dir_menu_nu_ui",
      "marker": "# ",
      "only_buffer_difference": true,
      "style": {
        "text": "#66ff66",
        "selected_text": {
          "fg": "#66ff66",
          "attr": "r"
        },
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": "{|buffer, position|\n        ls $env.PWD | where type == dir\n        | sort-by name | get name | to text\n        | fzf -f $buffer\n        | each {|v| {value: ($v | str trim)} }\n      }"
    },
    {
      "name": "commands_menu",
      "marker": "# ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "columnar",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2
      },
      "source": "{|buffer, position|\n        scope commands\n        | where name =~ $buffer\n        | each {|it| {value: $it.name description: $it.usage} }\n      }"
    },
    {
      "name": "vars_menu",
      "marker": "V ",
      "only_buffer_difference": true,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": "{|buffer, position|\n        scope variables\n        | where name =~ $buffer\n        | sort-by name\n        | each {|it| {value: $it.name description: $it.type} }\n      }"
    },
    {
      "name": "commands_with_description",
      "marker": "# ",
      "only_buffer_difference": true,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "description",
        "columns": 4,
        "col_width": 20,
        "col_padding": 2,
        "selection_rows": 4,
        "description_rows": 10
      },
      "source": "{|buffer, position|\n        scope commands\n        | where name =~ $buffer\n        | each {|it| {value: $it.name description: $it.usage} }\n      }"
    },
    {
      "name": "abbr_menu",
      "marker": "👀 ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "columnar",
        "columns": 1,
        "col_width": 20,
        "col_padding": 2
      },
      "source": "{|buffer, position|\n        scope aliases\n        | where name == $buffer\n        | each {|it| {value: $it.expansion} }\n      }"
    },
    {
      "name": "history_menu_by_session",
      "marker": "# ",
      "only_buffer_difference": false,
      "style": {
        "text": "green",
        "selected_text": "green_reverse",
        "description_text": "yellow"
      },
      "type": {
        "layout": "list",
        "page_size": 10
      },
      "source": "{|buffer, position|\n        history -l\n        | where session_id == (history session)\n        | select command\n        | where command =~ $buffer\n        | each {|it| {value: $it.command} }\n        | reverse\n        | uniq\n      }"
    }
  ],
  "hooks": {
    "pre_prompt": [
      "{|| null }",
      "{||\n  zoxide add -- $env.PWD\n}"
    ],
    "pre_execution": [
      "{|| null }"
    ],
    "env_change": {
      "PWD": [
        "{|before, after|\n          print (lsg)\n          # null\n        }",
        "{|before, _|\n          if $before == null {\n            let file = ($nu.home-path | path join \".local\" \"share\" \"nushell\" \"startup-times.nuon\")\n            if not ($file | path exists) {\n              mkdir ($file | path dirname)\n              touch $file\n            }\n            let ver = (version)\n            open $file | append {\n              date: (date now)\n              time: $nu.startup-time\n              build: ($ver.build_rust_channel)\n              allocator: ($ver.allocator)\n              version: ($ver.version)\n              commit: ($ver.commit_hash)\n              build_time: ($ver.build_time)\n              bytes_loaded: (view files | get size | math sum)\n            } | collect { save --force $file }\n          }\n        }",
        {
          "condition": "{|_, after| not ($after | path join 'toolkit.nu' | path exists) }",
          "code": "hide toolkit"
        },
        {
          "condition": "{|_, after| $after | path join 'toolkit.nu' | path exists }",
          "code": "\n        print $'(ansi default_underline)(ansi default_bold)toolkit(ansi reset) module (ansi green_italic)detected(ansi reset)...'\n        print $'(ansi yellow_italic)activating(ansi reset) (ansi default_underline)(ansi default_bold)toolkit(ansi reset) module with `(ansi default_dimmed)(ansi default_italic)use toolkit.nu(ansi reset)`'\n        use toolkit.nu\n        "
        }
      ]
    },
    "display_output": "{||\n      # if (term size).columns > 100 { table -e } else { table }\n      table\n    }",
    "command_not_found": "{||\n      null # return an error message when a command is not found\n    }"
  },
  "rm": {
    "always_trash": true
  },
  "shell_integration": {
    "osc2": true,
    "osc7": true,
    "osc8": true,
    "osc9_9": true,
    "osc133": true,
    "osc633": true,
    "reset_application_mode": true
  },
  "buffer_editor": "nvim",
  "show_banner": true,
  "bracketed_paste": true,
  "render_right_prompt_on_last_line": false,
  "explore": {
    "try": {
      "reactive": true
    },
    "table": {
      "selected_cell": {
        "bg": "blue"
      },
      "show_cursor": false
    }
  },
  "cursor_shape": {
    "emacs": "underscore",
    "vi_insert": "block",
    "vi_normal": "line"
  },
  "datetime_format": {
    "normal": null,
    "table": null
  },
  "error_style": "fancy",
  "display_errors": {
    "exit_code": true,
    "termination_signal": true
  },
  "use_kitty_protocol": true,
  "highlight_resolved_externals": true,
  "plugins": {},
  "plugin_gc": {
    "default": {
      "enabled": true,
      "stop_after": 0
    },
    "plugins": {
      "gstat": {
        "enabled": true,
        "stop_after": 0
      }
    }
  }
}"##;
    serde_json::from_str(json_str).expect("Failed to parse example JSON")
}
