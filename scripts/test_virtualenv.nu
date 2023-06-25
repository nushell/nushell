let env_name = 'e-$ èрт🚒♞中片-j'

let test_lines = [
    "python -c 'import sys; print(sys.executable)'"
    "python -c 'import os; import sys; v = os.environ.get("VIRTUAL_ENV"); print(v)'"
    $"overlay use '([$env.PWD $env_name bin activate.nu] | path join)'"
    "python -c 'import sys; print(sys.executable)'"
    "python -c 'import os; import sys; v = os.environ.get("VIRTUAL_ENV"); print(v)'"
    "print $env.VIRTUAL_PROMPT"
    # "pydoc -w pydoc_test"
    "deactivate"
    "python -c 'import sys; print(sys.executable)'"
    "python -c 'import os; import sys; v = os.environ.get("VIRTUAL_ENV"); print(v)'"
]

def make-error [] {
}

def main [] {
    let orig_python_interpreter = (python -c 'import sys; print(sys.executable)')

    let expected = [
        $orig_python_interpreter
        "None"
        ([$env.PWD $env_name bin python] | path join)
        ([$env.PWD $env_name] | path join)
        $"(char lparen)($env_name)(char rparen)"
        $orig_python_interpreter
        "None"
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
}
