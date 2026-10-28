const { readFileSync, writeFileSync } = require("node:fs")

const path = process.argv[2] ?? "crates/acta-napi/package.json"
const version = process.env.VERSION
if (!version) throw new Error("VERSION is not set")
const pkg = JSON.parse(readFileSync(path, "utf8"))
pkg.version = version
writeFileSync(path, `${JSON.stringify(pkg, null, 2)}\n`)
