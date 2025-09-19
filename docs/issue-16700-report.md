# Issue 16700 Report

## Summary
- **Problem**: `view source` failed for aliases defined inside closures/blocks, reporting "Cannot view string value".
- **Root Cause**: The command only searched the engine state's active overlays when resolving names, ignoring runtime-only definitions tracked in the evaluation stack.
- **Solution**: Extend name resolution to consult overlays active in the current stack and, when needed, fall back to recently defined declarations to retrieve local aliases/defs.
- **Tests**: Added an integration test covering closure aliases and verified with targeted `cargo test`.

## Phenomenon & Reproduction
Running the reproduction snippet from the issue demonstrates the failure:

```nu
> do { alias a = print; a 'alias is alive'; view source a }
alias is alive
Error: nu::shell::error

  × Cannot view string value
```

The alias invocation succeeds, but `view source` cannot find its definition.

## Root Cause Analysis
- `view source` used `EngineState::find_decl` to locate definitions. This searches only overlays tracked globally in the engine state.
- Aliases/definitions created inside closures are scoped dynamically. Their declarations exist in the runtime stack's overlay order but are not registered in the engine state's active overlay map.
- As a result, `view source` could not resolve such names and returned the generic error.

## Fix Implementation
- Introduced helper lookup that:
  1. Walks the overlays recorded in the current `Stack`, respecting visibility to resolve the declaration ID.
  2. If the overlay lookup fails (e.g. for stack-local definitions without overlay entries), fall back to scanning the declaration list for the latest matching name.
- This mirrors runtime resolution order while still using existing metadata for source retrieval.

## Alternatives Considered
- **Expose stack overlays directly**: Would require broader refactors in stack/scoping logic.
- **Register closure definitions in engine overlays**: Risked changing visibility semantics globally.
- The chosen approach is minimally invasive and leverages existing data already stored for declarations.

## Testing Strategy
- New integration test `view_source_alias_inside_closure` ensures closure-scoped aliases expose their source.
- Verified via `cargo test -p nu-command view_source_alias_inside_closure`.

