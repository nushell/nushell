$env.config = {
  hooks: {
    env_change: {
      PWD: [{|before, after|
        print -e $'Config Hook Called after cd to ($after)'
      }]
    }
  }
}

print 'Config start'

def-env add-pwd-hook [hook] {
  $env.config = ($env.config | upsert hooks.env_change.PWD {|config|
    let existing_hooks = ($config | get -i hooks.env_change.PWD)

    if $existing_hooks != $nothing {
        $existing_hooks | append $hook
    } else {
      [ $hook ]
    }
  })
}


print -e $'Initialized config'
overlay use ./overlay_test.nu

print -e $'Overlay Added'
