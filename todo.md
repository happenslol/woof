# Todo

## Core

- [x] Implement interpolation codegen
- [x] Sanitize translation keys and module names (e.g. `delete`)
- [x] Implement namespaces
- [ ] Allow configuring default locale
- [ ] Fix key overlaps after sanitization

## Reporting

- [x] Collect errors instead of aborting parsing
- [x] Report spanned errors using miette
- [ ] Detect missing translations
- [ ] Use `supports-colors` to disable colors conditionally

## Build

- [ ] Configure static binary build
- [ ] Build binaries in CI
