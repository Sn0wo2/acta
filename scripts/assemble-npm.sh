set -euo pipefail
shopt -s nullglob
linux=(dist/*_napi_*_linux-windows.tar.gz)
macos=(dist/*_napi_*_macos.tar.gz)
[[ ${#linux[@]} == 1 && ${#macos[@]} == 1 ]]
mkdir -p "$RUNNER_TEMP/napi-linux" "$RUNNER_TEMP/napi-macos" "$RUNNER_TEMP/napi-package"
tar -xzf "${linux[0]}" -C "$RUNNER_TEMP/napi-linux"
tar -xzf "${macos[0]}" -C "$RUNNER_TEMP/napi-macos"
cp "$RUNNER_TEMP/napi-linux/"{index.js,index.d.ts,package.json} "$RUNNER_TEMP/napi-package/"
for file in acta_napi.linux-x64-gnu.node acta_napi.linux-arm64-gnu.node acta_napi.win32-x64-msvc.node acta_napi.win32-arm64-msvc.node; do
  cp "$RUNNER_TEMP/napi-linux/$file" "$RUNNER_TEMP/napi-package/"
done
for file in acta_napi.darwin-x64.node acta_napi.darwin-arm64.node; do
  cp "$RUNNER_TEMP/napi-macos/$file" "$RUNNER_TEMP/napi-package/"
done
cp LICENSE "$RUNNER_TEMP/napi-package/"
bun scripts/sync-napi-version.js "$RUNNER_TEMP/napi-package/package.json"
cd "$RUNNER_TEMP/napi-package"
bun -e 'require("./index.js")'
bun pm pack --dry-run
