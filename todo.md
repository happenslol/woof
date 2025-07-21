# Todo

## Core

- [x] Implement interpolation codegen
- [x] Sanitize translation keys and module names (e.g. `delete`)
- [x] Implement namespaces
- [ ] Allow configuring default locale

## Reporting

- [ ] Collect errors instead of aborting parsing
- [ ] Detect missing translations
- [ ] Report spanned errors using ariadne/miette

## Benchmarks

Not critical _at all_, but might be fun.

- [ ] Keep string references to original toml file
- [ ] Pre-allocate as much as possible
- [ ] Avoid string joining

## Build

- [ ] Configure static binary build
- [ ] Build binaries in CI

## Bugs

- [ ] Correctly remove escaped braces from interpolations (`{{` -> `{`)
