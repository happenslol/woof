#!/usr/bin/env zx

const tag = await $`git tag --points-at HEAD`
if (!tag) {
  console.error("No tag found")
  process.exit(1)
}

const version = tag.toString().trim().replace(/^v/, "")
console.log(`Releasing version ${version}`)

await fs.remove("./release")
await fs.mkdirp("./release")

const commonPackageJson = {
  version,
  license: "MIT",
  preferUnplugged: true,
  repository: {
    type: "git",
    url: "git+https://github.com/happenslol/woof.git"
  },
}

const platformPackages = [
  {
    artifact: "woof-x86_64-unknown-linux-musl",
    bin: "woof",
    os: "linux",
    cpu: "x64",
  },
  {
    artifact: "woof-x86_64-apple-darwin",
    bin: "woof",
    os: "darwin",
    cpu: "x64",
  },
  {
    artifact: "woof-aarch64-apple-darwin",
    bin: "woof",
    os: "darwin",
    cpu: "arm64",
  },
  {
    artifact: "woof-x86_64-pc-windows-msvc.exe",
    bin: "woof.exe",
    os: "win32",
    cpu: "x64",
  },
]

console.log("Generating platform packages")

await Promise.all(platformPackages.map(async p => {
  const target = `${p.os}-${p.cpu}`

  const packageJson = {
    ...commonPackageJson,
    name: `@woofcli/${target}`,
    os: [p.os],
    cpu: [p.cpu],
    description: `woof binary for ${target}`,
  }

  await fs.mkdirp(`./release/${target}/bin`)
  await fs.writeFile(
    `./release/${target}/package.json`,
    JSON.stringify(packageJson, null, 2)
  )

  const res = await fetch(`https://github.com/happenslol/woof/releases/download/${tag}/${p.artifact}`)
  const buf = await res.arrayBuffer()
  const binPath = `./release/${target}/bin/${p.bin}`
  await fs.writeFile(binPath, Buffer.from(buf))
  await $`chmod +x ${binPath}`

  await fs.writeFile(`./release/${target}/README.md`, `# woof

This is the ${target} binary for woof. See [the repository](https://github.com/happenslol/woof) for details.`)
}))

console.log("Generating main package")

const optionalDependencies = platformPackages.reduce((acc, p) => {
  acc[`@woofcli/${p.os}-${p.cpu}`] = version
  return acc
}, {})

const packageJson = {
  ...commonPackageJson,
  name: "@woofcli/woof",
  bin: { woof: "bin/woof" },
  optionalDependencies,
}

await fs.mkdirp("./release/woof")
await fs.writeFile(
  "./release/woof/package.json",
  JSON.stringify(packageJson, null, 2)
)

await fs.copy("./README.md", "./release/woof/README.md")
await fs.copy("./npm/node-shim.js", "./release/woof/bin/woof")
