# ANSI Command Fixes and Improvements

## Summary

This PR addresses several issues with the `ansi` command and improves error handling throughout the codebase.

## Changes Made

### 1. Fixed ANSI Attribute Parsing Bug
- **Issue**: The `ansi` command had incorrect color reversal when using the `strike` attribute
- **Fix**: Corrected the attribute parsing logic in `fill_modifiers()` function to properly handle ANSI style attributes
- **Files**: `crates/nu-color-config/src/nu_style.rs`

### 2. Enhanced Error Handling
- **Issue**: Invalid ANSI attributes were silently ignored instead of providing helpful error messages
- **Fix**: Added comprehensive error handling with descriptive messages for both invalid attribute codes and names
- **Files**: `crates/nu-color-config/src/nu_style.rs`, `crates/nu-command/src/strings/ansi/ansi_.rs`

### 3. Removed Unsafe unwrap/expect Calls
- **Issue**: Code contained several `unwrap()` and `expect()` calls that could panic
- **Fix**: Replaced with safe alternatives using `unwrap_or()`, `unwrap_or_else()`, and proper error propagation
- **Files**: `crates/nu-color-config/src/nu_style.rs`, `crates/nu-command/src/strings/ansi/ansi_.rs`

### 4. Code Quality Improvements
- **Issue**: Code duplication and inconsistent error messages
- **Fix**: Extracted constants for valid attributes, created helper functions, and improved maintainability
- **Files**: `crates/nu-color-config/src/nu_style.rs`

## Examples

### Before/After: Strike Attribute Fix

**Before (buggy behavior):**
```bash
# This would incorrectly reverse colors when using strike
$ ansi --escape { fg: "#ff0000" bg: "#000000" attr: "strike" }
```

**After (correct behavior):**
```bash
# Now correctly applies strike-through without color reversal
$ ansi --escape { fg: "#ff0000" bg: "#000000" attr: "strike" }
```

### Error Handling Improvements

**Invalid attribute codes now show helpful errors:**
```bash
$ ansi --escape { fg: "#ff0000" attr: "x" }
Error: nu::shell::error
  × Invalid ANSI attribute code
  help: Valid codes are: b (bold), i (italic), u (underline), s (strike), d (dimmed), r (reverse), h (hidden), l (blink), n (normal)
```

**Invalid attribute names now show helpful errors:**
```bash
$ ansi --escape { fg: "#ff0000" attr: "invalid" }
Error: nu::shell::error
  × Invalid ANSI attribute name
  help: Valid names are: bold, italic, underline, strike, dimmed, reverse, hidden, blink, normal
```

### Code Examples

**Valid attribute combinations work correctly:**
```bash
# Multiple attributes as codes
$ ansi --escape { fg: "#ff0000" attr: "biu" }  # bold + italic + underline

# Multiple attributes as names
$ ansi --escape { fg: "#00ff00" attr: "bold italic" }

# Mixed codes and names
$ ansi --escape { fg: "#0000ff" attr: "b,underline" }
```

## Testing

- All existing tests pass
- Manual testing confirms proper error messages for invalid attributes
- ANSI command works correctly with valid attributes
- Clippy passes with no warnings

## Breaking Changes

None. All changes maintain backward compatibility.

## Release Notes

- Fixed: `ansi` command now correctly handles the `strike` attribute without color reversal
- Improved: Better error messages when invalid ANSI attributes are provided
- Enhanced: More robust error handling throughout the ANSI styling system