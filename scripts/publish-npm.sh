set -euo pipefail
cd "$RUNNER_TEMP/napi-package"
bun -e '
  const pkg = await Bun.file("package.json").json();
  const response = await fetch(`https://registry.npmjs.org/${encodeURIComponent(pkg.name)}/${encodeURIComponent(pkg.version)}`);
  if (response.ok) process.exit(0);
  if (response.status === 404) process.exit(2);
  throw new Error(`npm version lookup failed: ${response.status}`);
' && { echo "Version already published; skipping."; exit 0; } || status=$?
[[ $status == 2 ]] || exit "$status"
if [[ "$PRERELEASE" == true ]]; then
  npm publish --tag next
else
  npm publish
fi
