#!/usr/bin/env node
"use strict"

const os = require("node:os")

const packageName = `${process.platform}-${os.arch()}`
const binaryName = process.platform === "win32" ? "woof.exe" : "woof"

let binPath
try {
  binPath = require.resolve(`@woofcli/${packageName}/bin/${binaryName}`)
} catch (e) {
  throw new Error(`woof binary not found: ${e}`)
}

try {
  require("child_process").execFileSync(
    binPath,
    process.argv.slice(2),
    { stdio: "inherit" }
  )
} catch (e) {
  process.exit(e.status || 1)
}
