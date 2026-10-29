#!/usr/bin/env bun
const { createHash } = require("node:crypto")
const { spawnSync } = require("node:child_process")
const { copyFileSync, mkdirSync, rmSync } = require("node:fs")
const { resolve } = require("node:path")

const rustc = spawnSync("rustc", ["-vV"], { encoding: "utf8" })
if (rustc.error || rustc.status !== 0) {
  console.error(`ERROR: rustc -vV failed: ${rustc.error?.message ?? rustc.stderr}`)
  process.exit(1)
}
const host = rustc.stdout?.match(/host:\s+(\S+)/)?.[1]
if (!host) {
  console.error("ERROR: Could not determine host target from rustc -vV")
  process.exit(1)
}

const args = process.argv.slice(2)
const targetIndex = args.findIndex((arg) => arg === "--target" || arg.startsWith("--target="))
const targetArg = args[targetIndex]
const target = targetArg === "--target" ? args[targetIndex + 1] : targetArg?.slice(9)
const cargoTarget = target?.endsWith(".glibc") ? target.slice(0, -6) : target
if (cargoTarget !== target) {
  if (targetArg === "--target") args[targetIndex + 1] = cargoTarget
  else args[targetIndex] = `--target=${cargoTarget}`
}
let env = process.env
if (args[0] === "build" || args[0] === "zigbuild") args.shift()
let command = "build"
if (cargoTarget === "x86_64-pc-windows-gnu" && cargoTarget !== host) {
  env = {
    ...env,
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER: process.env.CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER ?? "x86_64-w64-mingw32-gcc",
  }
} else if (cargoTarget && cargoTarget !== host) {
  env = {
    ...env,
    CARGO_ZIGBUILD_CACHE_DIR: resolve("target", "zigbuild", cargoTarget, createHash("sha256").update(process.env.RUSTFLAGS ?? "").digest("hex")),
  }
  command = "zigbuild"
}
args.unshift(command)

const debugProfile = process.env.BUILD_PROFILE === "debug"
if (debugProfile) {
  const buildArgs = []
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--release" || args[i] === "-r" || args[i].startsWith("--profile=")) continue
    if (args[i] === "--profile") {
      i++
      continue
    }
    buildArgs.push(args[i])
  }
  buildArgs.push("--profile=dev")
  args.splice(0, args.length, ...buildArgs)
}

const binIndex = args.findIndex((arg) => arg === "--bin" || arg.startsWith("--bin="))
const bin = args[binIndex] === "--bin" ? args[binIndex + 1] : args[binIndex]?.slice(6)
const targetPrefix = target?.split(".", 1)[0]
const targetAbi = targetPrefix?.slice(targetPrefix.lastIndexOf("-") + 1)
const outputTarget = target && targetAbi?.startsWith("gnu") && targetAbi !== "gnullvm" ? targetPrefix : target
if (debugProfile && (!bin || !outputTarget)) {
  console.error("ERROR: debug builds require a binary name and target")
  process.exit(1)
}
const outputName = `${bin ?? "acta-debug"}${cargoTarget?.includes("windows") ? ".exe" : ""}`
const releasePath = outputTarget && resolve("target", outputTarget, "release", outputName)
if (debugProfile && releasePath) rmSync(releasePath, { force: true })

const result = spawnSync("cargo", args, {
  stdio: "inherit",
  env,
})

if (debugProfile && result.status === 0 && outputTarget) {
  const debugPath = resolve("target", outputTarget, "debug", outputName)
  try {
    mkdirSync(resolve("target", outputTarget, "release"), { recursive: true })
    copyFileSync(debugPath, releasePath)
  } catch (error) {
    console.error(`ERROR: Could not stage debug executable: ${error.message}`)
    process.exit(1)
  }
}

process.exit(result.status ?? 1)
