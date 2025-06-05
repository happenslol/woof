# Woof

`woof` is a simple code generator for translations.

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

With the files `locales/en.toml` and `locales/de.toml`, running `woof -o messages ./locales` will generate the following:

```typescript
// messages/index.ts
let _locale = "en"

export const setLocale = (l: "en" | "de") => (_locale = l)
export const locale = () => _locale

export * as m from "./root"

// messages/root.ts
import { locale } from "."

export const title = () => {
  const l = locale()
  if (l === "en") return "My Website"
  if (l === "de") return "Meine Webseite"
  return "title"
}

export const description = () => {
  const l = locale()
  if (l === "en") return "This is a description"
  if (l === "de") return "Dies ist eine Beschreibung"
  return "description"
}

export * as about from "./about"

// messages/about/index.ts
import { locale } from ".."

export const title = () => {
  const l = locale()
  if (l === "en") return "About"
  if (l === "de") return "Über"
  return "about.title"
}

export const description = () => {
  const l = locale()
  if (l === "en") return "More details about this website"
  if (l === "de") return "Mehr Details über diese Webseite"
  return "about.description"
}

export * as more from "./more"

// messages/about/more/index.ts
import { locale } from "../.."

export const copyright = (year: number, author: string) => {
  const l = locale()
  if (l === "en") return `Copyright ${year} by ${author}`
  if (l === "de") return `Copyright ${year} von ${author}`
  return `about.more.copyright (year:${year}, author:${author})`
}
```

You can then use the translations in your code like this:

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
