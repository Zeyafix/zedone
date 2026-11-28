import assert from "node:assert/strict";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const env = { ...process.env, CARGO_TARGET_DIR: resolve(root, "target") };
const versions = await Bun.file(resolve(root, "scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun);
const mode = Bun.argv[2];
assert.ok(Bun.argv.length <= 3 && [undefined, "--main", "--conflicts"].includes(mode));
function run(args) {
  console.log(`> ${args.join(" ")}`);
  const result = Bun.spawnSync(args, { cwd: root, env, stdin: "ignore", stdout: "inherit", stderr: "inherit" });
  assert.ok(!result.signalCode, `Command interrupted: ${args[0]}`);
  assert.equal(result.exitCode, 0, args.join(" "));
}
const nodePath = process.env.UI_NODE_PATH || "node";
const node = Bun.spawnSync([nodePath, "--version"]);
const nodeVersion = process.env.UI_NODE_VERSION || versions.node;
assert.equal(node.stdout.toString().trim(), `v${nodeVersion}`, "vue-tsc Node runtime mismatch");
console.log(`vue-tsc runtime Node ${nodeVersion}; repository baseline ${versions.node}`);
if (mode !== "--conflicts") {
  run([process.execPath, "test", resolve(root, "tests/qr-scanner.test.ts")]);
  run([process.execPath, "test", resolve(root, "tests/snapshot-files.test.ts")]);
  run([nodePath, "node_modules/vue-tsc/bin/vue-tsc.js", "-p", "ui/tsconfig.json"]);
}
run([process.execPath, "run", "ui:build"]);
if (mode !== "--conflicts") for (const args of [
  ["fmt", "--", "--check"],
  ["clippy", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"],
  ["test", "--locked", "--offline"],
]) run(["cargo", `+${versions.rust}`, args[0], "--manifest-path", "ui/src-tauri/Cargo.toml", ...args.slice(1)]);
run(["cargo", `+${versions.rust}`, "build", "--manifest-path", "ui/src-tauri/Cargo.toml", "--bin", "serve-shell", "--example", "shell-fixture", "--example", "maintenance-fixture", "--locked", "--offline"]);
if (mode !== "--conflicts") run(["cargo", `+${versions.rust}`, "build", "-p", "zedone-adapters", "--bin", "zedone", "--locked", "--offline"]);
run([process.execPath, "scripts/check-ui-browser.mjs", mode || "--main"]);
if (mode !== "--conflicts") run([process.execPath, "scripts/check-ui-maintenance.mjs"]);
if (!mode) run([process.execPath, "scripts/check-ui-browser.mjs", "--conflicts"]);
if (mode !== "--main") run([process.execPath, "scripts/check-ui-pairing.mjs"]);
