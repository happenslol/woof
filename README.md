# Woof

Experimenting with codegen for translations

## Notes

- Inspired by inlang/paraglide-js
  - Codegen is a great idea, but they use a binary translation file
  - Can we do the same, but have a stupid simple, version control friendly, text editable file format?
- Is tree-shaking essential?
  - Nested keys that are called as `nested.key()` are not possible if we want tree-shaking
  - inlang generates files that export `*` from the other translation files, nested keys are exported as string literals and have to be called as `m['nested.key']()`
- Functions are needed because the locale switching has to happen inside the translation function
- We could allow switching between JS and react mode, in react mode we just use fragments to allow interpolating components into translations
  - Do we need to provide components for translations? e.g. `<nested.key.C value="v" />`?
