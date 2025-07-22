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

## Build

- [ ] Configure static binary build
- [ ] Build binaries in CI

## Bugs

- [x] Correctly remove escaped braces from interpolations (`{{` -> `{`)
