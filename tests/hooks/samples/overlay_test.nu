export-env {
  if (not ($env | default false _hooked | get _hooked)) {
    $env._hooked = true
    $env.config = ($env | default {} config).config
    $env.config = ($env.config | default {} hooks)
    $env.config = ($env.config | update hooks ($env.config.hooks | default {} env_change))
    $env.config = ($env.config | update hooks.env_change ($env.config.hooks.env_change | default [] PWD))
    $env.config = ($env.config | update hooks.env_change.PWD ($env.config.hooks.env_change.PWD | append {|_, dir|
      print -e 'Overlay Called yay!'
      $env.TEST_RESULT = 'overlay hook';
    }))
  }
}
