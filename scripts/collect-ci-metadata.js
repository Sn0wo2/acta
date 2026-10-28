const { appendFileSync } = require("node:fs")
const { execFileSync } = require("node:child_process")

const { packages } = JSON.parse(execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], { encoding: "utf8" }))
const hasBinaries = packages.some((pkg) => pkg.targets.some((target) => target.kind.includes("bin")))
const hasLibrary = packages.some((pkg) => pkg.targets.some((target) => target.kind.includes("lib")))
appendFileSync(process.env.GITHUB_OUTPUT, `has_binaries=${hasBinaries}\nhas_library=${hasLibrary}\n`)
