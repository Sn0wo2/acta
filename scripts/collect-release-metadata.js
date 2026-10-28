const { appendFileSync } = require("node:fs")
const { execFileSync } = require("node:child_process")

const { packages } = JSON.parse(execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], { encoding: "utf8" }))
const binaries = packages.filter((pkg) => pkg.targets.some((target) => target.kind.includes("bin")))
const publishable = packages.filter((pkg) => pkg.publish == null || pkg.publish.includes("crates-io"))
if (process.env.VERIFY_VERSION === "true") {
  const mismatches = [...new Set([...binaries, ...publishable])].filter((pkg) => pkg.version !== process.env.VERSION)
  if (mismatches.length) throw new Error(`Expected released packages to have version ${process.env.VERSION}`)
}
const sha = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim()
if (process.env.GITHUB_EVENT_NAME === "push" && sha !== process.env.GITHUB_SHA) throw new Error("Checkout does not match the pushed tag")
appendFileSync(process.env.GITHUB_OUTPUT, `has_binaries=${binaries.length > 0}\nhas_publishable_crates=${publishable.length > 0}\nsha=${sha}\n`)
