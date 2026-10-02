#!/usr/bin/env bun
import { existsSync, mkdirSync, appendFileSync, unlinkSync } from "node:fs"

const SDK_URL = "https://github.com/phracker/MacOSX-SDKs/releases/download/11.3/MacOSX11.3.sdk.tar.xz"
const SDK_ROOT = "/opt/MacOSX11.3.sdk"
const TMP_PATH = "/tmp/MacOSX11.3.sdk.tar.xz"

if (existsSync(SDK_ROOT)) {
  console.log(`SDK already installed at ${SDK_ROOT}`)
} else {
  console.log(`Downloading macOS SDK from ${SDK_URL}`)
  const res = await fetch(SDK_URL)
  if (!res.ok) {
    console.error(`ERROR: download failed: ${res.status} ${res.statusText}`)
    process.exit(1)
  }
  await Bun.write(TMP_PATH, res)

  console.log("Extracting macOS SDK")
  mkdirSync("/opt", { recursive: true })
  await Bun.$`tar -xJf ${TMP_PATH} -C /opt`
  unlinkSync(TMP_PATH)

  if (!existsSync(SDK_ROOT)) {
    console.error(`ERROR: SDK extraction failed, ${SDK_ROOT} not found`)
    process.exit(1)
  }
  console.log(`macOS SDK installed at ${SDK_ROOT}`)
}

const envFile = process.env.GITHUB_ENV
if (envFile) {
  appendFileSync(envFile, `SDKROOT=${SDK_ROOT}\n`)
  console.log(`SDKROOT=${SDK_ROOT} written to GITHUB_ENV`)
}
