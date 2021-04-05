# Nu-Engine
Nu-engine handles most of the core logic of nushell. For example, engine handles:
    - Passing of data between commands
    - Evaluating a commands return values
    - Loading of user configurations

## Top level introduction
The following topics shall give the reader a top level understanding how various topics are handled in nushell.

### How are environment variables handled?
Environment variables (or short envs) are stored in the `Scope` of the `EvaluationContext`. That means that environment variables are scoped by default and we don't use `std::env` to store envs (but make exceptions where convenient).

Nushell handles environment variables and their lifetime the following:
- At startup all existing environment variables are read and put into `Scope`. (Nushell reads existing environment variables platform independent by asking the `Host`. They will most likely come from `std::env::*`)
- Envs can also be loaded from config files. Each loaded config produces a new `ScopeFrame` with the envs of the loaded config.
- Nu-Script files and internal commands read and write env variables from / to the `Scope`. External scripts and binaries can't interact with the `Scope`. Therefore all env variables are read from the `Scope` and put into the external binaries environment-variables-memory area.
