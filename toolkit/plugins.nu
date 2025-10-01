def windows? [] {
    $nu.os-info.name == windows
}

# filter out files that end in .d
def keep-plugin-executables [] {
    if (windows?) { where name ends-with '.exe' } else { where name !~ '\.d' }
}

# add all installed plugins
export def "add plugins" [] {
    let plugin_path = (which nu | get path.0 | path dirname)
    let plugins = (ls $plugin_path | where name =~ nu_plugin | keep-plugin-executables | get name)

    if ($plugins | is-empty) {
        print $"no plugins found in ($plugin_path)..."
        return
    }

    for plugin in $plugins {
        try {
            print $"> plugin add ($plugin)"
            plugin add $plugin
        } catch { |err|
            print -e $"(ansi rb)Failed to add ($plugin):\n($err.msg)(ansi reset)"
        }
    }

    print $"\n(ansi gb)plugins registered, please restart nushell(ansi reset)"
}
