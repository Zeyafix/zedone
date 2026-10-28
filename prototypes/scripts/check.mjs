import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
const mode = Bun.argv[2];
if (Bun.argv.length > 3 || (mode && !["--write-openapi", "--check-openapi"].includes(mode))) {
  console.error("Expected no arguments, --write-openapi or --check-openapi.");
  process.exit(1);
}

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

function cargo(args, capture = false) {
  const cmd = ["cargo", `+${versions.rust}`, ...args];
  console.log(`> ${cmd.join(" ")}`);
  const result = Bun.spawnSync(cmd, {
    cwd: root,
    env,
    stdin: "ignore",
    stdout: capture ? "pipe" : "inherit",
    stderr: "inherit",
  });
  if (result.exitCode !== 0 || result.signalCode) {
    console.error(`Prototype check failed: cargo ${args[0]}.`);
    process.exit(result.exitCode || 1);
  }
  return result.stdout;
}

if (!mode) {
  for (const args of checks) cargo(args);
}

// Capture first: compilation or generation failure must leave the artifact intact.
const generated = cargo([
  "run", "--quiet", "--locked", "--offline", "--bin", "generate-openapi",
], true);
const artifact = Bun.file(resolve(root, "openapi.json"));
if (mode === "--write-openapi") {
  await Bun.write(artifact, generated);
  console.log("Generated openapi.json from Rust; review its diff.");
} else {
  if (!(await artifact.exists()) || !generated.equals(Buffer.from(await artifact.arrayBuffer()))) {
    console.error("openapi.json is missing or stale. Run bun run generate:openapi and review the diff.");
    process.exit(1);
  }
  console.log("OpenAPI matches Rust. HTTP, client and Android remain unverified.");
}
