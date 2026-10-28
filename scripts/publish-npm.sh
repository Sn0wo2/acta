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
printf '%s\n' '//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}' > "$RUNNER_TEMP/npmrc"
export NPM_CONFIG_USERCONFIG="$RUNNER_TEMP/npmrc"
if [[ "$PRERELEASE" == true ]]; then
  bun publish --tag next
else
  bun publish
fi
