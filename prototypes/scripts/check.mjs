import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();

if (Bun.version !== versions.bun) {
  console.error(`Expected Bun ${versions.bun}; found ${Bun.version}.`);
  process.exit(1);
}

// Pin the build output to this workspace even if the caller has a global override.
const env = { ...process.env, CARGO_TARGET_DIR: resolve(root, "target") };
const checks = [
  ["fmt", "--all", "--", "--check"],
  ["clippy", "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"],
  ["test", "--workspace", "--locked", "--offline"],
  ["build", "--workspace", "--release", "--locked", "--offline"],
];

for (const args of checks) {
  const cmd = ["cargo", `+${versions.rust}`, ...args];
  console.log(`> ${cmd.join(" ")}`);
  const result = Bun.spawnSync(cmd, {
    cwd: root,
    env,
    stdin: "ignore",
    stdout: "inherit",
    stderr: "inherit",
  });
  if (result.exitCode !== 0 || result.signalCode) {
    console.error(`Prototype check failed: cargo ${args[0]}.`);
    process.exit(result.exitCode || 1);
  }
}

console.log("Prototype foundation checks passed; no application scenarios were exercised.");
