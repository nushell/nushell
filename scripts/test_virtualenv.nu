
let test_lines = [
    "python -c 'import sys; print(sys.executable)'"
    "python -c 'import os; import sys; v = os.environ.get("VIRTUAL_ENV"); print(v)'"
    "overlay use 'spam/bin/activate.nu'"
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
    error make {msg: (which python)}
    let orig_python_interpreter = (which python).path.0
    let expected = [
        $orig_python_interpreter
        "None"
        $"($env.PWD)/spam/bin/python"
        $"($env.PWD)/spam"
        "(spam)"
        $orig_python_interpreter
        "None"
    ]

    virtualenv spam

    $test_lines | save script.nu
    let out = (nu script.nu | lines)

    let o = ($out | str trim | str join (char nl))
    let e = ($expected | str trim | str join (char nl))
    if $o != $e {
        print $"OUTPUT:\n($o)\n\nEXPECTED:\n($e)"
        error make {msg: "Output does not match the expected value"}
    }
}
