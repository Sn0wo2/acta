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
const target = args[targetIndex] === "--target" ? args[targetIndex + 1] : args[targetIndex]?.slice(9)
let env = process.env
if (args[0] === "build" || args[0] === "zigbuild") args.shift()
let command = "build"
if (target === "x86_64-pc-windows-gnu" && target !== host) {
  env = {
    ...env,
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER: process.env.CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER ?? "x86_64-w64-mingw32-gcc",
  }
} else if (target && target !== host) {
  env = {
    ...env,
    CARGO_ZIGBUILD_CACHE_DIR: resolve("target", "zigbuild", target, createHash("sha256").update(process.env.RUSTFLAGS ?? "").digest("hex")),
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
const abi = targetPrefix?.slice(targetPrefix.lastIndexOf("-") + 1)
const outputTarget = target && abi?.startsWith("gnu") && abi !== "gnullvm" ? targetPrefix : target
if (debugProfile && (!bin || !outputTarget)) {
  console.error("ERROR: debug builds require a binary name and target")
  process.exit(1)
}
const outputName = `${bin}${target?.includes("windows") ? ".exe" : ""}`
const releasePath = outputTarget && resolve("target", outputTarget, "release", outputName)
if (debugProfile && releasePath) rmSync(releasePath, { force: true })

const result = spawnSync("cargo", args, { stdio: "inherit", env })

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
