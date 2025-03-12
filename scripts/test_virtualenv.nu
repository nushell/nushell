if $nu.os-info.family == 'windows' {
    # fix encoding on Windows https://stackoverflow.com/a/63573649
    load-env {
        PYTHONIOENCODING: utf-8
        PYTHONLEGACYWINDOWSSTDIO: utf-8
    }
}

let env_name = 'e-$ èрт🚒♞中片-j'

let paths = if $nu.os-info.family == 'windows' {
    ['Scripts', 'python.exe']
} else {
    ['bin', 'python']
}

let subdir = $paths.0
let exe = $paths.1

let test_lines = [
    "python -c 'import sys; print(sys.executable)'"                                  # 1
    `python -c 'import os; import sys; v = os.environ.get("VIRTUAL_ENV"); print(v)'` # 2
    $"overlay use '([$env.PWD $env_name $subdir activate.nu] | path join)'"
    "python -c 'import sys; print(sys.executable)'"                                  # 3
    `python -c 'import os; import sys; v = os.environ.get("VIRTUAL_ENV"); print(v)'` # 4
    "print $env.VIRTUAL_ENV_PROMPT"                                                  # 5
    "deactivate"
    "python -c 'import sys; print(sys.executable)'"                                  # 6
    `python -c 'import os; import sys; v = os.environ.get("VIRTUAL_ENV"); print(v)'` # 7
]

def main [] {
    let orig_python_interpreter = (python -c 'import sys; print(sys.executable)')

    let expected = [
        $orig_python_interpreter                           # 1
        "None"                                             # 2
        ([$env.PWD $env_name $subdir $exe] | path join)    # 3
        ([$env.PWD $env_name] | path join)                 # 4
        $env_name                                          # 5
        $orig_python_interpreter                           # 6
        "None"                                             # 7
    ]

    virtualenv $env_name

    $test_lines | save script.nu
    let out = (nu script.nu | lines)

    let o = ($out | str trim | str join (char nl))
    let e = ($expected | str trim | str join (char nl))
    if $o != $e {
        let msg = $"OUTPUT:\n($o)\n\nEXPECTED:\n($e)"
        error make {msg: $"Output does not match the expected value:\n($msg)"}
    }
    rm script.nu
}
