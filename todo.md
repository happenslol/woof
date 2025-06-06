# Todo

## Core

- [ ] Implement interpolation codegen
- [ ] Sanitize translation keys and module names (e.g. `delete`)
- [ ] Implement namespaces
- [ ] Allow configuring default locale

## Reporting

- [ ] Collect errors instead of aborting parsing
- [ ] Detect translation key/module name collisions
- [ ] Detect missing translations
- [ ] Report spanned errors using [ariadne](https://docs.rs/ariadne/latest/ariadne/)

## Benchmarks

Not critical _at all_, but might be fun.

- [ ] Writing bit-by-bit vs concatting string and writing all
- [ ] Keep string references to original toml file
- [ ] Pre-allocate as much as possible
- [ ] Avoid string joining

## Build

- [ ] Configure static binary build
- [ ] Build binaries in CI
