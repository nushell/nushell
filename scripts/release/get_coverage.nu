# Test coverage gathering for nushell 
# Uses cargo-llvm-cov
# Uses separate execution to track the integration tests
# Hacked together by @sholderbach

# Get test coverage for nushell
def main [
       --extra # Get coverage for extra features
   ] {
   cargo llvm-cov show-env --export-prefix | 
       lines | 
       str substring 7.. | 
       split column '=' | 
       str trim -c '"'  | 
       transpose | 
       headers | 
       reject 'column1' | 
       get 0 | 
       str trim |
       load-env
   
   cargo llvm-cov clean --workspace 
   if $extra {
       cargo build --workspace --features extra
       cargo test --workspace --features extra
   } else {
       cargo build --workspace
       cargo test --workspace
   }
   cargo llvm-cov --no-run --lcov --output-path lcov.info 
   cargo llvm-cov --no-run --html
}
