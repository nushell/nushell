def --env main [stem: string] {
    let in_dir = $env.PWD
    
    echo $"in_dir = ($in_dir)"
    
    cd '~/Templates/dev'

    echo $"after cd, PWD = ($env.PWD)"
    # let template = ^ls -1
    #     | sk -i --ansi -c 'fd {}' --preview 'bat -pp --color=always {}'
    #     | str trim -r
    let template = 'foo.rs'

    echo $template

    let extension = $template | path parse | get extension
    let dest = $in_dir | path join $"($stem).($extension)"

    echo $"Executing 'cp ($template) ($dest)'"
    cp ($template) ($dest)
}
