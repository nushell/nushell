use std assert

# Parameter name:
# sig type   : any
# name       : command
# type       : positional
# shape      : string
# description: external command to run

# Parameter name:
# sig type   : any
# name       : args
# type       : rest
# shape      : any
# description: arguments for external command

# Parameter name:
# sig type   : any
# name       : redirect-stdout
# type       : switch
# shape      : 
# description: redirect stdout to the pipeline

# Parameter name:
# sig type   : any
# name       : redirect-stderr
# type       : switch
# shape      : 
# description: redirect stderr to the pipeline

# Parameter name:
# sig type   : any
# name       : redirect-combine
# type       : switch
# shape      : 
# description: redirect both stdout and stderr combined to the pipeline (collected in stdout)

# Parameter name:
# sig type   : any
# name       : trim-end-newline
# type       : switch
# shape      : 
# description: trimming end newlines


# This is the custom command 1 for run-external:

#[test]
def run-external_run_an_external_command_1 [] {
  let result = (run-external "echo" "-n" "hello")
  assert ($result == )
}

# This is the custom command 2 for run-external:

#[test]
def run-external_redirect_stdout_from_an_external_command_into_the_pipeline_2 [] {
  let result = (run-external --redirect-stdout "echo" "-n" "hello" | split chars)
  assert ($result == )
}


