import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
if (Bun.version !== versions.bun) throw new Error(`Expected Bun ${versions.bun}`);
const options = {
  cwd: root,
  env: { ...process.env, CARGO_TARGET_DIR: resolve(root, "target") },
  stdin: "ignore", stdout: "inherit", stderr: "inherit",
};
const build = Bun.spawnSync(["cargo", `+${versions.rust}`, "build", "--locked", "--offline", "--bin", "serve-echo"], options);
if (build.exitCode !== 0) process.exit(build.exitCode || 1);
const child = Bun.spawn([resolve(root, `target/debug/serve-echo${process.platform === "win32" ? ".exe" : ""}`)], options);
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => child.kill());
process.exit(await child.exited);
