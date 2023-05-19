## run the scripts

> **Note**  
> the following table must be read as follows:
> - an `x` means *it works*
> - a `?` means *no data available*
>
> `.nu` scripts must be run as `nu .../foo.nu`
> `.sh` scripts must be run as `./.../foo.sh`
> `.ps1` scripts must be run as `powershell .../foo.ps1`
>
> let's say a script is called `foo`
> - `./scripts`: `foo` can be run from `./scripts`
> - root: `foo` can be run from the root of `nushell`
> - anywhere: `foo` can be run from anywhere!

| script                  | `./scripts/` | root | anywhere |
| ----------------------- | ------------ | ---- | -------- |
| `build-all-maclin.sh`   | x            | x    | x        |
| `build-all-windows.cmd` | ?            | x    | ?        |
| `build-all.nu`          | x            | x    | x        |
| `coverage-local.nu`     | x            | x    | x        |
| `coverage-local.sh`     | x            | x    | x        |
| `install-all.ps1`       | ?            | x    | ?        |
| `install-all.sh`        | x            | x    | x        |
| `register-plugins.nu`   | x            | x    | x        |
| `uninstall-all.sh`      | x            | x    | x        |
