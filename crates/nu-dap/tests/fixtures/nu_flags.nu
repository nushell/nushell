# Fixture for the `$nu` flags a debug session must report: stdout is the DAP
# wire, so this is a protocol server and not an interactive shell.
print $"is-dap=($nu.is-dap) is-interactive=($nu.is-interactive)"
