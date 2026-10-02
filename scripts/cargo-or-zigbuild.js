#!/usr/bin/env bun
import { spawnSync } from "node:child_process"

const rustc = spawnSync("rustc", ["-vV"], { encoding: "utf8" })
if (rustc.status !== 0) {
  console.error(`ERROR: rustc -vV failed: ${rustc.stderr}`)
  process.exit(1)
}
const host = rustc.stdout.match(/host:\s+(\S+)/)?.[1]
if (!host) {
  console.error("ERROR: Could not determine host target from rustc -vV")
  process.exit(1)
}

let args = process.argv.slice(2)
let target = null
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--target" && i + 1 < args.length) {
    target = args[i + 1]
    break
  }
  if (args[i].startsWith("--target=")) {
    target = args[i].slice("--target=".length)
    break
  }
}

let command = ["cargo"]
if (target && target !== host) {
  command = ["cargo", "zigbuild"]
  if (args[0] === "build") args = args.slice(1)
}

const result = spawnSync(command[0], [...command.slice(1), ...args], { stdio: "inherit" })
process.exit(result.status ?? 1)
