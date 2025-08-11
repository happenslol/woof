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

- [x] Configure static binary build
- [x] Build binaries in CI
- [ ] Replace binary in postinstall script
- [ ] Check version when running
- [ ] Throw incompatible platform error
