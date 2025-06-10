use std/dt [datetime-diff, pretty-print-duration]

# Print a banner for Nushell with information about the project
@category "default"
@search-terms "welcome" "startup"
export def banner [
    --short    # Only show startup time
] {
let foreground = $env.config.color_config?.banner_foreground? | default "attr_normal"
let highlight1 = $env.config.color_config?.banner_highlight1? | default "green"
let highlight2 = $env.config.color_config?.banner_highlight2? | default "purple"
let dt = (datetime-diff (date now) 2019-05-10T09:59:12-07:00)
let ver = (version)
let startup_time = $"(ansi $highlight1)(ansi attr_bold)Startup Time: (ansi reset)(ansi $foreground)($nu.startup-time)(ansi reset)"

let banner_msg = match $short {
    true => $"($startup_time)(char eol)"

    false => $"(ansi $highlight1)     __  ,(ansi reset)
(ansi $highlight1) .--\(\)°'.' (ansi reset)(ansi $foreground)Welcome to (ansi $highlight1)Nushell(ansi reset)(ansi $foreground),(ansi reset)
(ansi $highlight1)'|, . ,'   (ansi reset)(ansi $foreground)based on the (ansi $highlight1)nu(ansi reset)(ansi $foreground) language,(ansi reset)
(ansi $highlight1) !_-\(_\\    (ansi reset)(ansi $foreground)where all data is structured!

(ansi $foreground)Version: (ansi $highlight1)($ver.version) \(($ver.build_target)\)(ansi reset)
(ansi $foreground)Please join our (ansi $highlight2)Discord(ansi reset)(ansi $foreground) community at (ansi $highlight2)https://discord.gg/NtAbbGn(ansi reset)
(ansi $foreground)Our (ansi $highlight1)(ansi attr_bold)GitHub(ansi reset)(ansi $foreground) repository is at (ansi $highlight1)(ansi attr_bold)https://github.com/nushell/nushell(ansi reset)
(ansi $foreground)Our (ansi $highlight2)Documentation(ansi reset)(ansi $foreground) is located at (ansi $highlight2)https://nushell.sh(ansi reset)
(ansi $foreground)And the (ansi $highlight1)Latest Nushell News(ansi reset)(ansi $foreground) at (ansi $highlight1)https://nushell.sh/blog/(ansi reset)
(ansi $foreground)Learn how to remove this at: (ansi $highlight2)https://nushell.sh/book/configuration.html#remove-welcome-message(ansi reset)

(ansi $foreground)It's been this long since (ansi $highlight1)Nushell(ansi reset)(ansi $foreground)'s first commit:(ansi reset)
(ansi $foreground)(pretty-print-duration $dt)

($startup_time)(ansi reset)
"
}

match (config use-colors) {
    false => { $banner_msg | ansi strip }
    _ => $banner_msg
}
}

# Return the current working directory
@category "default"
export def pwd [
    --physical (-P) # Resolve symbolic links
] {
    if $physical {
        $env.PWD | path expand
    } else {
        $env.PWD
    }
}
