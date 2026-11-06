import assert from "node:assert/strict";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
// Share product artifacts with product-check. Cargo separates host/Android
// targets and profiles, and rebuilds units whose features or flags differ.
const env = { ...process.env, CARGO_TARGET_DIR: resolve(root, "target") };
const versions = await Bun.file(resolve(root, "scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
function run(args) {
  console.log(`> ${args.join(" ")}`);
  const result = Bun.spawnSync(args, { cwd: root, env, stdin: "ignore", stdout: "inherit", stderr: "inherit" });
  assert.equal(result.exitCode, 0, args.join(" "));
}
const nodePath = process.env.UI_NODE_PATH || "node";
const node = Bun.spawnSync([nodePath, "--version"]);
// An explicit local override records a differing prepared environment; CI uses
// the repository baseline. Neither changes the delivered browser runtime.
const nodeVersion = process.env.UI_NODE_VERSION || versions.node;
assert.equal(node.stdout.toString().trim(), `v${nodeVersion}`, "vue-tsc Node runtime mismatch");
console.log(`vue-tsc runtime Node ${nodeVersion}; repository baseline ${versions.node}`);
run([nodePath, "node_modules/vue-tsc/bin/vue-tsc.js", "-p", "ui/tsconfig.json"]);
run([process.execPath, "run", "ui:build"]);
for (const args of [
  ["fmt", "--", "--check"],
  ["clippy", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"],
  ["test", "--locked", "--offline"],
  ["build", "--bin", "serve-shell", "--example", "shell-fixture", "--locked", "--offline"],
]) run(["cargo", `+${versions.rust}`, args[0], "--manifest-path", "ui/src-tauri/Cargo.toml", ...args.slice(1)]);
run([process.execPath, "scripts/check-ui-browser.mjs"]);
