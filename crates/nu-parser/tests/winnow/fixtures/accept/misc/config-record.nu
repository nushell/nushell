$env.config = {
  show_banner: false
  ls: { use_ls_colors: true }
  hooks: {
    pre_prompt: [{ null }]
    env_change: { PWD: [{|before, after| null }] }
    display_output: "if (term size).columns >= 100 { table -e } else { table }"
  }
  menus: [
    { name: completion_menu, only_buffer_difference: false, marker: "| " }
  ]
  keybindings: [
    { name: x, modifier: none, keycode: char_x, mode: [emacs vi_normal], event: { send: enter } }
  ]
  color_config: { command_bar_text: { fg: '#C4C9C6' }, }
  rm: { always_trash: false, }
}
