# to-yaml

Converts table data into yaml text.

## Example

```shell
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path 
───┼───┼────────────┼────────────────────────
 0 │ X │ filesystem │ /home/shaurya 
 1 │   │ filesystem │ /home/shaurya/Pictures 
 2 │   │ filesystem │ /home/shaurya/Desktop 
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━
> shells | to-yaml
---
- " ": X
  name: filesystem
  path: /home/shaurya
- " ": " "
  name: filesystem
  path: /home/shaurya/Pictures
- " ": " "
  name: filesystem
  path: /home/shaurya/Desktop
```

```shell
> open appveyor.yml 
━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━┯━━━━━━━┯━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━
 image              │ environment    │ install         │ build │ test_script     │ cache 
────────────────────┼────────────────┼─────────────────┼───────┼─────────────────┼─────────────────
 Visual Studio 2017 │ [table: 1 row] │ [table: 5 rows] │       │ [table: 2 rows] │ [table: 2 rows] 
━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━┷━━━━━━━┷━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━
> open appveyor.yml | to-yaml
---
image: Visual Studio 2017
environment:
  global:
    PROJECT_NAME: nushell
    RUST_BACKTRACE: 1
  matrix:
    - TARGET: x86_64-pc-windows-msvc
      CHANNEL: nightly
      BITS: 64
install:
  - "set PATH=C:\\msys64\\mingw%BITS%\\bin;C:\\msys64\\usr\\bin;%PATH%"
  - "curl -sSf -o rustup-init.exe https://win.rustup.rs"
  - rustup-init.exe -y --default-host %TARGET% --default-toolchain %CHANNEL%-%TARGET%
  - "set PATH=%PATH%;C:\\Users\\appveyor\\.cargo\\bin"
  - "call \"C:\\Program Files (x86)\\Microsoft Visual Studio\\2017\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat\""
build: false
test_script:
  - cargo build --verbose
  - cargo test --all --verbose
cache:
  - target -> Cargo.lock
  - "C:\\Users\\appveyor\\.cargo\\registry -> Cargo.lock"
```
