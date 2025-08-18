# Woof

`woof` is a simple translation code generator.

## How it works

`woof` takes `.toml` files as input and generates typescript files with functions than you can call to get your translation strings.

All string interpolation is typesafe and the resulting code is tree-shakable, so you'll only ship the translations you're actually using. Lazy-loading can be taken care of by your bundler, in case you have a lot of translations.

## Usage

Here's what the translation files look like:

```toml
title = "My Website"
description = "This is a description"

[about]
title = "About"
description = "More details about this website"

[about.more]
copyright = "Copyright {year:number} by {author}"
```

With the files `locales/en.toml` and `locales/de.toml`, running `woof -o messages ./locales` will generate message files that you can use like this:

```typescript
import { m } from './messages'

console.log(m.about.title()) // "About"
console.log(m.about.more.copyright({ year: 2022, author: 'me' })) // "Copyright 2022 by me"
```

You can also directory import messages from a sub-directory:

```typescript
import * as m from './messages/about/more'

console.log(m.copyright({ year: 2022, author: 'me' })) // "Copyright 2022 by me"
```

## Setting the Locale

Instead of having a global locale variable, you define a getter that will be used by translations. By default, this is set to a function that returns the default locale.

This allows you to have a global variable, use `AsyncLocalStorage`, or any other mechanism you'd like:

```typescript
import { setLocaleFn } from "./messages"

// Get it from local storage
setLocaleFn(() => localStorage.getItem("locale") || "en")

// Use a global variable
let locale = "en"
setLocaleFn(() => locale)

// Use async local storage in node
const localeStorage = new AsyncLocalStorage()
setLocaleFn(() => localeStorage.getStore() ?? "en")
```
